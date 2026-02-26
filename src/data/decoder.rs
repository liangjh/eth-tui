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
