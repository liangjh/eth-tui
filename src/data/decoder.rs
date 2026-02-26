use alloy::dyn_abi::{DynSolValue, JsonAbiExt};
use alloy::json_abi::JsonAbi;
use alloy::primitives::{Address, Bytes, B256, U256};

use crate::data::types::{DecodedCall, TokenTransfer};

/// The keccak256 hash of `Transfer(address,address,uint256)`.
/// This is the topic0 for ERC-20 Transfer events.
const TRANSFER_EVENT_TOPIC: B256 = {
    // 0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef
    B256::new([
        0xdd, 0xf2, 0x52, 0xad, 0x1b, 0xe2, 0xc8, 0x9b, 0x69, 0xc2, 0xb0, 0x68, 0xfc, 0x37,
        0x8d, 0xaa, 0x95, 0x2b, 0xa7, 0xf1, 0x63, 0xc4, 0xa1, 0x16, 0x28, 0xf5, 0x5a, 0x4d,
        0xf5, 0x23, 0xb3, 0xef,
    ])
};

pub struct TxDecoder;

impl TxDecoder {
    /// Decode function call input data using a known ABI.
    ///
    /// Attempts to match the 4-byte selector in `input` against every function
    /// in the ABI. If a match is found, the remaining calldata is ABI-decoded
    /// and the result is returned as a `DecodedCall`.
    pub fn decode_input(abi: &JsonAbi, input: &Bytes) -> Option<DecodedCall> {
        if input.len() < 4 {
            return None;
        }
        let selector: [u8; 4] = input[..4].try_into().ok()?;

        for func in abi.functions() {
            if func.selector() == selector {
                // Try to decode the calldata parameters
                match func.abi_decode_input(&input[4..], false) {
                    Ok(values) => {
                        let params: Vec<(String, String)> = func
                            .inputs
                            .iter()
                            .zip(values.iter())
                            .map(|(param, value): (&alloy::json_abi::Param, &DynSolValue)| {
                                (param.name.clone(), format_sol_value(value))
                            })
                            .collect();

                        return Some(DecodedCall {
                            function_name: func.name.clone(),
                            params,
                        });
                    }
                    Err(_) => {
                        // Selector matched but decoding failed; return the function
                        // name without decoded parameters.
                        return Some(DecodedCall {
                            function_name: func.name.clone(),
                            params: vec![],
                        });
                    }
                }
            }
        }

        None
    }

    /// Extract ERC-20 `Transfer` events from raw transaction logs.
    ///
    /// A standard ERC-20 Transfer log has:
    /// - topic[0] = keccak256("Transfer(address,address,uint256)")
    /// - topic[1] = from address (zero-padded to 32 bytes)
    /// - topic[2] = to address (zero-padded to 32 bytes)
    /// - data     = value (uint256, 32 bytes)
    pub fn extract_token_transfers(logs: &[alloy::rpc::types::Log]) -> Vec<TokenTransfer> {
        let mut transfers = Vec::new();

        for log in logs {
            let topics = log.inner.data.topics();
            let data = log.inner.data.data.as_ref();

            // Must have exactly 3 topics for ERC-20 Transfer
            if topics.len() != 3 {
                continue;
            }

            // Check the event signature
            if topics[0] != TRANSFER_EVENT_TOPIC {
                continue;
            }

            // Parse from and to addresses from topics (last 20 bytes of each 32-byte topic)
            let from = Address::from_slice(&topics[1].as_slice()[12..]);
            let to = Address::from_slice(&topics[2].as_slice()[12..]);

            // Parse value from data (first 32 bytes)
            let value = if data.len() >= 32 {
                U256::from_be_slice(&data[..32])
            } else {
                U256::ZERO
            };

            let token_address = log.inner.address;

            transfers.push(TokenTransfer {
                token_address,
                from,
                to,
                value,
                token_name: None,
                token_symbol: None,
                decimals: None,
            });
        }

        transfers
    }

    /// Extract the 4-byte method selector from transaction input data.
    pub fn extract_selector(input: &Bytes) -> Option<[u8; 4]> {
        if input.len() < 4 {
            return None;
        }
        let mut selector = [0u8; 4];
        selector.copy_from_slice(&input[..4]);
        Some(selector)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::{LogData, Log as PrimitiveLog};

    #[test]
    fn test_extract_selector_too_short() {
        let input = Bytes::from(vec![0xa9, 0x05, 0x9c]);
        assert!(TxDecoder::extract_selector(&input).is_none());
    }

    #[test]
    fn test_extract_selector_empty() {
        let input = Bytes::new();
        assert!(TxDecoder::extract_selector(&input).is_none());
    }

    #[test]
    fn test_extract_selector_exact_4_bytes() {
        let input = Bytes::from(vec![0xa9, 0x05, 0x9c, 0xbb]);
        assert_eq!(
            TxDecoder::extract_selector(&input),
            Some([0xa9, 0x05, 0x9c, 0xbb])
        );
    }

    #[test]
    fn test_extract_selector_longer_input() {
        let input = Bytes::from(vec![0xa9, 0x05, 0x9c, 0xbb, 0x00, 0x01, 0x02]);
        assert_eq!(
            TxDecoder::extract_selector(&input),
            Some([0xa9, 0x05, 0x9c, 0xbb])
        );
    }

    #[test]
    fn test_decode_input_too_short() {
        let abi: JsonAbi = serde_json::from_str("[]").unwrap();
        let input = Bytes::from(vec![0x01, 0x02]);
        assert!(TxDecoder::decode_input(&abi, &input).is_none());
    }

    fn erc20_functions_abi() -> JsonAbi {
        // Minimal ABI with just the functions (no events that need `anonymous` field)
        let json = r#"[
            {"type":"function","name":"transfer","inputs":[{"name":"to","type":"address"},{"name":"amount","type":"uint256"}],"outputs":[{"name":"","type":"bool"}],"stateMutability":"nonpayable"},
            {"type":"function","name":"approve","inputs":[{"name":"spender","type":"address"},{"name":"amount","type":"uint256"}],"outputs":[{"name":"","type":"bool"}],"stateMutability":"nonpayable"}
        ]"#;
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn test_decode_input_erc20_transfer() {
        let abi = erc20_functions_abi();

        // Build calldata for transfer(address,uint256)
        // selector: 0xa9059cbb
        let mut calldata = vec![0xa9, 0x05, 0x9c, 0xbb];
        // address param (padded to 32 bytes)
        let mut addr_bytes = vec![0u8; 12];
        let to_addr = Address::from_slice(&[0xd8, 0xdA, 0x6B, 0xF2, 0x69, 0x64, 0xaF, 0x9D,
                                            0x7e, 0xEd, 0x9e, 0x03, 0xE5, 0x34, 0x15, 0xD3,
                                            0x7a, 0xA9, 0x60, 0x45]);
        addr_bytes.extend_from_slice(to_addr.as_slice());
        calldata.extend_from_slice(&addr_bytes);
        // uint256 param: 1000 (big-endian)
        let mut amount = vec![0u8; 32];
        amount[30] = 0x03;
        amount[31] = 0xe8; // 0x03e8 = 1000
        calldata.extend_from_slice(&amount);

        let input = Bytes::from(calldata);
        let decoded = TxDecoder::decode_input(&abi, &input).unwrap();
        assert_eq!(decoded.function_name, "transfer");
        assert_eq!(decoded.params.len(), 2);
        assert_eq!(decoded.params[0].0, "to");
        assert_eq!(decoded.params[1].0, "amount");
        assert_eq!(decoded.params[1].1, "1000");
    }

    #[test]
    fn test_decode_input_unknown_selector() {
        let abi = erc20_functions_abi();

        // Unknown selector
        let input = Bytes::from(vec![0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00]);
        assert!(TxDecoder::decode_input(&abi, &input).is_none());
    }

    fn make_transfer_log(
        token: Address,
        from: Address,
        to: Address,
        value: U256,
    ) -> alloy::rpc::types::Log {
        let mut from_topic = B256::ZERO;
        from_topic.0[12..].copy_from_slice(from.as_slice());
        let mut to_topic = B256::ZERO;
        to_topic.0[12..].copy_from_slice(to.as_slice());

        let mut data_bytes = vec![0u8; 32];
        value.to_be_bytes::<32>().iter().enumerate().for_each(|(i, b)| {
            data_bytes[i] = *b;
        });

        let log_data = LogData::new(
            vec![TRANSFER_EVENT_TOPIC, from_topic, to_topic],
            Bytes::from(data_bytes),
        ).unwrap();

        alloy::rpc::types::Log {
            inner: PrimitiveLog {
                address: token,
                data: log_data,
            },
            block_hash: None,
            block_number: None,
            block_timestamp: None,
            transaction_hash: None,
            transaction_index: None,
            log_index: None,
            removed: false,
        }
    }

    #[test]
    fn test_extract_token_transfers_valid() {
        let token = Address::from_slice(&[0x01; 20]);
        let from = Address::from_slice(&[0x02; 20]);
        let to = Address::from_slice(&[0x03; 20]);
        let value = U256::from(1000u64);

        let logs = vec![make_transfer_log(token, from, to, value)];
        let transfers = TxDecoder::extract_token_transfers(&logs);

        assert_eq!(transfers.len(), 1);
        assert_eq!(transfers[0].token_address, token);
        assert_eq!(transfers[0].from, from);
        assert_eq!(transfers[0].to, to);
        assert_eq!(transfers[0].value, value);
        assert!(transfers[0].token_name.is_none());
    }

    #[test]
    fn test_extract_token_transfers_wrong_topic_count() {
        // Only 2 topics instead of 3 — should be skipped
        let log_data = LogData::new(
            vec![TRANSFER_EVENT_TOPIC, B256::ZERO],
            Bytes::new(),
        ).unwrap();
        let log = alloy::rpc::types::Log {
            inner: PrimitiveLog {
                address: Address::ZERO,
                data: log_data,
            },
            block_hash: None,
            block_number: None,
            block_timestamp: None,
            transaction_hash: None,
            transaction_index: None,
            log_index: None,
            removed: false,
        };

        let transfers = TxDecoder::extract_token_transfers(&[log]);
        assert!(transfers.is_empty());
    }

    #[test]
    fn test_extract_token_transfers_wrong_event_signature() {
        let log_data = LogData::new(
            vec![B256::ZERO, B256::ZERO, B256::ZERO],
            Bytes::from(vec![0u8; 32]),
        ).unwrap();
        let log = alloy::rpc::types::Log {
            inner: PrimitiveLog {
                address: Address::ZERO,
                data: log_data,
            },
            block_hash: None,
            block_number: None,
            block_timestamp: None,
            transaction_hash: None,
            transaction_index: None,
            log_index: None,
            removed: false,
        };

        let transfers = TxDecoder::extract_token_transfers(&[log]);
        assert!(transfers.is_empty());
    }

    #[test]
    fn test_extract_token_transfers_empty_logs() {
        let transfers = TxDecoder::extract_token_transfers(&[]);
        assert!(transfers.is_empty());
    }
}

/// Format a dynamic Solidity value to a human-readable string.
fn format_sol_value(value: &DynSolValue) -> String {
    match value {
        DynSolValue::Bool(b) => b.to_string(),
        DynSolValue::Int(i, _) => i.to_string(),
        DynSolValue::Uint(u, _) => u.to_string(),
        DynSolValue::FixedBytes(b, _) => format!("0x{}", alloy::primitives::hex::encode(b)),
        DynSolValue::Address(a) => format!("{a}"),
        DynSolValue::Function(f) => format!("0x{}", alloy::primitives::hex::encode(f)),
        DynSolValue::Bytes(b) => {
            if b.len() <= 32 {
                format!("0x{}", alloy::primitives::hex::encode(b))
            } else {
                format!(
                    "0x{}... ({} bytes)",
                    alloy::primitives::hex::encode(&b[..32]),
                    b.len()
                )
            }
        }
        DynSolValue::String(s) => format!("\"{s}\""),
        DynSolValue::Array(arr) | DynSolValue::FixedArray(arr) => {
            let inner: Vec<String> = arr.iter().map(format_sol_value).collect();
            format!("[{}]", inner.join(", "))
        }
        DynSolValue::Tuple(parts) => {
            let inner: Vec<String> = parts.iter().map(format_sol_value).collect();
            format!("({})", inner.join(", "))
        }
    }
}
