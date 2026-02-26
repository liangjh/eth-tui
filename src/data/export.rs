use std::fs;
use std::io::Write;

use crate::data::types::{AddressInfo, BlockSummary, TransactionDetail};

/// Export block summaries to CSV format.
///
/// Columns: number, hash, timestamp, tx_count, gas_used, gas_limit, base_fee, miner
pub fn export_blocks_csv(blocks: &[BlockSummary], path: &str) -> Result<String, String> {
    let file = fs::File::create(path).map_err(|e| format!("Failed to create file: {e}"))?;
    let mut wtr = csv::Writer::from_writer(file);

    // Write header
    wtr.write_record([
        "number",
        "hash",
        "timestamp",
        "tx_count",
        "gas_used",
        "gas_limit",
        "base_fee_wei",
        "miner",
        "eth_burned_wei",
    ])
    .map_err(|e| format!("Failed to write CSV header: {e}"))?;

    // Write rows
    for block in blocks {
        wtr.write_record(&[
            block.number.to_string(),
            format!("{:#x}", block.hash),
            block.timestamp.to_string(),
            block.tx_count.to_string(),
            block.gas_used.to_string(),
            block.gas_limit.to_string(),
            block.base_fee.map(|f| f.to_string()).unwrap_or_default(),
            format!("{:#x}", block.miner),
            block
                .eth_burned
                .map(|b| b.to_string())
                .unwrap_or_default(),
        ])
        .map_err(|e| format!("Failed to write CSV row: {e}"))?;
    }

    wtr.flush().map_err(|e| format!("Failed to flush CSV: {e}"))?;

    Ok(format!("Exported {} blocks to {path}", blocks.len()))
}

/// Export transaction detail to JSON format.
pub fn export_tx_json(detail: &TransactionDetail, path: &str) -> Result<String, String> {
    let json = serde_json::json!({
        "hash": format!("{:#x}", detail.summary.hash),
        "block_number": detail.summary.block_number,
        "from": format!("{:#x}", detail.summary.from),
        "to": detail.summary.to.map(|a| format!("{:#x}", a)),
        "value_wei": detail.summary.value.to_string(),
        "nonce": detail.nonce,
        "gas_limit": detail.gas_limit,
        "gas_used": detail.summary.gas_used,
        "gas_price": detail.summary.gas_price,
        "max_fee_per_gas": detail.max_fee_per_gas,
        "max_priority_fee_per_gas": detail.max_priority_fee_per_gas,
        "effective_gas_price": detail.effective_gas_price,
        "status": detail.summary.status.to_string(),
        "tx_type": detail.summary.tx_type.to_string(),
        "method_name": detail.summary.method_name,
        "input_data": format!("0x{}", alloy::primitives::hex::encode(&detail.input_data)),
        "decoded_input": detail.decoded_input.as_ref().map(|d| serde_json::json!({
            "function": d.function_name,
            "params": d.params.iter().map(|(name, val)| serde_json::json!({
                "name": name,
                "value": val,
            })).collect::<Vec<_>>(),
        })),
        "token_transfers": detail.token_transfers.iter().map(|t| serde_json::json!({
            "token": format!("{:#x}", t.token_address),
            "from": format!("{:#x}", t.from),
            "to": format!("{:#x}", t.to),
            "value": t.value.to_string(),
            "token_name": t.token_name,
            "token_symbol": t.token_symbol,
            "decimals": t.decimals,
        })).collect::<Vec<_>>(),
        "logs_count": detail.logs_count,
        "confirmations": detail.confirmations,
    });

    let formatted = serde_json::to_string_pretty(&json)
        .map_err(|e| format!("Failed to serialize JSON: {e}"))?;

    let mut file = fs::File::create(path).map_err(|e| format!("Failed to create file: {e}"))?;
    file.write_all(formatted.as_bytes())
        .map_err(|e| format!("Failed to write file: {e}"))?;

    Ok(format!("Exported transaction to {path}"))
}

/// Export address info to JSON format.
pub fn export_address_json(info: &AddressInfo, path: &str) -> Result<String, String> {
    let json = serde_json::json!({
        "address": format!("{:#x}", info.address),
        "balance_wei": info.balance.to_string(),
        "nonce": info.nonce,
        "is_contract": info.is_contract,
        "contract_info": info.contract_info.as_ref().map(|c| serde_json::json!({
            "abi_source": c.abi_source,
            "is_proxy": c.is_proxy,
            "implementation": c.implementation.map(|a| format!("{:#x}", a)),
            "contract_type": c.contract_type.map(|t| t.to_string()),
            "name": c.name,
            "symbol": c.symbol,
            "decimals": c.decimals,
        })),
        "recent_transactions": info.transactions.iter().map(|tx| serde_json::json!({
            "hash": format!("{:#x}", tx.hash),
            "block_number": tx.block_number,
            "from": format!("{:#x}", tx.from),
            "to": tx.to.map(|a| format!("{:#x}", a)),
            "value_wei": tx.value.to_string(),
            "status": tx.status.to_string(),
        })).collect::<Vec<_>>(),
    });

    let formatted = serde_json::to_string_pretty(&json)
        .map_err(|e| format!("Failed to serialize JSON: {e}"))?;

    let mut file = fs::File::create(path).map_err(|e| format!("Failed to create file: {e}"))?;
    file.write_all(formatted.as_bytes())
        .map_err(|e| format!("Failed to write file: {e}"))?;

    Ok(format!("Exported address info to {path}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::{Address, B256, U256};
    use std::fs;

    fn sample_blocks() -> Vec<BlockSummary> {
        vec![
            BlockSummary {
                number: 19000000,
                hash: B256::ZERO,
                timestamp: 1700000000,
                tx_count: 150,
                gas_used: 15_000_000,
                gas_limit: 30_000_000,
                base_fee: Some(30_000_000_000),
                miner: Address::ZERO,
                eth_burned: None,
            },
            BlockSummary {
                number: 19000001,
                hash: B256::ZERO,
                timestamp: 1700000012,
                tx_count: 200,
                gas_used: 20_000_000,
                gas_limit: 30_000_000,
                base_fee: Some(35_000_000_000),
                miner: Address::ZERO,
                eth_burned: Some(U256::from(700_000_000_000_000_000u64)),
            },
        ]
    }

    #[test]
    fn test_export_blocks_csv() {
        let blocks = sample_blocks();
        let path = "/tmp/eth-tui-test-blocks.csv";
        let result = export_blocks_csv(&blocks, path);
        assert!(result.is_ok());

        let contents = fs::read_to_string(path).unwrap();
        assert!(contents.contains("number"));
        assert!(contents.contains("19000000"));
        assert!(contents.contains("19000001"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_export_blocks_csv_empty() {
        let path = "/tmp/eth-tui-test-blocks-empty.csv";
        let result = export_blocks_csv(&[], path);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("0 blocks"));

        let _ = fs::remove_file(path);
    }
}
