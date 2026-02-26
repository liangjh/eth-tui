pub mod abi;
pub mod cache;
pub mod chains;
pub mod decoder;
pub mod ens;
pub mod export;
pub mod provider;
pub mod types;
pub mod watchlist;
pub mod ws;

use std::sync::Arc;

use alloy::consensus::Transaction as ConsensusTransaction;
use alloy::primitives::{Address, Bytes, B256, U256};
use alloy::rpc::types::{Block, Transaction, TransactionReceipt};
use alloy::sol;
use alloy::sol_types::SolCall;
use tokio::sync::mpsc;
use tokio::sync::RwLock;

use crate::data::abi::AbiResolver;
use crate::data::cache::DataCache;
use crate::data::decoder::TxDecoder;
use crate::data::provider::EthProvider;
use crate::data::types::*;
use crate::events::{AppEvent, SearchTarget, View};

// ERC-20 token ABI for metadata calls
sol! {
    #[derive(Debug)]
    interface IERC20Metadata {
        function name() external view returns (string);
        function symbol() external view returns (string);
        function decimals() external view returns (uint8);
    }
}

/// EIP-1967 implementation storage slot
const EIP1967_IMPL_SLOT: U256 = {
    U256::from_be_bytes([
        0x36, 0x08, 0x94, 0xa1, 0x3b, 0xa1, 0xa3, 0x21, 0x06, 0x67, 0xc8, 0x28, 0x49, 0x2d,
        0xb9, 0x8d, 0xca, 0x3e, 0x20, 0x76, 0xcc, 0x37, 0x35, 0xa9, 0x20, 0xa3, 0xca, 0x50,
        0x5d, 0x38, 0x2b, 0xbc,
    ])
};

pub struct DataService {
    provider: Arc<EthProvider>,
    cache: Arc<RwLock<DataCache>>,
    abi_resolver: Arc<AbiResolver>,
    event_tx: mpsc::UnboundedSender<AppEvent>,
    etherscan_api_key: Option<String>,
}

impl DataService {
    pub fn new(
        provider: EthProvider,
        etherscan_api_key: Option<String>,
        event_tx: mpsc::UnboundedSender<AppEvent>,
    ) -> Self {
        Self {
            provider: Arc::new(provider),
            cache: Arc::new(RwLock::new(DataCache::new())),
            abi_resolver: Arc::new(AbiResolver::new(etherscan_api_key.clone())),
            event_tx,
            etherscan_api_key,
        }
    }

    /// Fetch the latest block number and send it as an event.
    pub fn fetch_latest_block_number(&self) {
        let provider = Arc::clone(&self.provider);
        let tx = self.event_tx.clone();

        tokio::spawn(async move {
            match provider.get_latest_block_number().await {
                Ok(number) => {
                    let _ = tx.send(AppEvent::LatestBlockNumber(number));
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::Error(format!("Failed to get block number: {e}")));
                }
            }
        });
    }

    /// Fetch the most recent `count` blocks as summaries.
    pub fn fetch_recent_blocks(&self, count: usize) {
        let provider = Arc::clone(&self.provider);
        let cache = Arc::clone(&self.cache);
        let tx = self.event_tx.clone();

        tokio::spawn(async move {
            let latest = match provider.get_latest_block_number().await {
                Ok(n) => n,
                Err(e) => {
                    let _ = tx.send(AppEvent::Error(format!("Failed to get block number: {e}")));
                    return;
                }
            };

            let start = latest.saturating_sub(count as u64 - 1);
            let mut summaries = Vec::with_capacity(count);

            for number in (start..=latest).rev() {
                // Check cache first
                {
                    let mut c = cache.write().await;
                    if let Some(cached) = c.get_block(number) {
                        summaries.push(cached);
                        continue;
                    }
                }

                // Fetch from provider
                match provider.get_block(number).await {
                    Ok(Some(block)) => {
                        let summary = block_to_summary(&block);
                        {
                            let mut c = cache.write().await;
                            c.put_block(number, summary.clone());
                        }
                        summaries.push(summary);
                    }
                    Ok(None) => {
                        // Block not found (unlikely for recent blocks), skip
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::Error(format!(
                            "Failed to fetch block {number}: {e}"
                        )));
                    }
                }
            }

            let _ = tx.send(AppEvent::RecentBlocks(summaries));
        });
    }

    /// Fetch full block detail including transaction summaries.
    pub fn fetch_block_detail(&self, number: u64) {
        let provider = Arc::clone(&self.provider);
        let cache = Arc::clone(&self.cache);
        let tx = self.event_tx.clone();

        tokio::spawn(async move {
            // Check cache
            {
                let mut c = cache.write().await;
                if let Some(cached) = c.get_block_detail(number) {
                    let _ = tx.send(AppEvent::BlockDetailLoaded(Box::new(cached)));
                    return;
                }
            }

            // Fetch block with full transactions
            let block = match provider.get_block(number).await {
                Ok(Some(b)) => b,
                Ok(None) => {
                    let _ = tx.send(AppEvent::Error(format!("Block {number} not found")));
                    return;
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::Error(format!(
                        "Failed to fetch block {number}: {e}"
                    )));
                    return;
                }
            };

            // Fetch receipts for gas-used information
            let receipts = match provider.get_block_receipts(number).await {
                Ok(r) => r,
                Err(_) => vec![],
            };

            // Build receipt lookup by tx hash
            let receipt_map: std::collections::HashMap<B256, &TransactionReceipt> = receipts
                .iter()
                .map(|r| (r.transaction_hash, r))
                .collect();

            let summary = block_to_summary(&block);
            let timestamp = block.header.timestamp;

            // Build transaction summaries
            let transactions: Vec<TransactionSummary> = block
                .transactions
                .as_transactions()
                .map(|txs| {
                    txs.iter()
                        .map(|t| {
                            let tx_hash = *t.inner.tx_hash();
                            let receipt = receipt_map.get(&tx_hash).copied();
                            tx_to_summary(t, receipt, timestamp)
                        })
                        .collect()
                })
                .unwrap_or_default();

            let detail = BlockDetail {
                summary,
                parent_hash: block.header.parent_hash,
                state_root: block.header.state_root,
                size: block.header.size.map(|s| s.to::<u64>()),
                transactions,
                total_difficulty: block.header.total_difficulty,
            };

            {
                let mut c = cache.write().await;
                c.put_block_detail(number, detail.clone());
            }

            let _ = tx.send(AppEvent::BlockDetailLoaded(Box::new(detail)));
        });
    }

    /// Fetch full transaction detail with receipt, decoded input, and token transfers.
    pub fn fetch_transaction_detail(&self, hash: B256) {
        let provider = Arc::clone(&self.provider);
        let cache = Arc::clone(&self.cache);
        let abi_resolver = Arc::clone(&self.abi_resolver);
        let tx = self.event_tx.clone();

        tokio::spawn(async move {
            // Check cache
            {
                let mut c = cache.write().await;
                if let Some(cached) = c.get_transaction(hash) {
                    let _ = tx.send(AppEvent::TransactionDetailLoaded(Box::new(cached)));
                    return;
                }
            }

            // Fetch transaction
            let transaction = match provider.get_transaction(hash).await {
                Ok(Some(t)) => t,
                Ok(None) => {
                    let _ = tx.send(AppEvent::Error(format!("Transaction {hash} not found")));
                    return;
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::Error(format!(
                        "Failed to fetch transaction {hash}: {e}"
                    )));
                    return;
                }
            };

            // Fetch receipt
            let receipt = match provider.get_transaction_receipt(hash).await {
                Ok(r) => r,
                Err(_) => None,
            };

            // Get block timestamp if we have a block number
            let block_timestamp = if let Some(block_num) = transaction.block_number {
                match provider.get_block(block_num).await {
                    Ok(Some(b)) => b.header.timestamp,
                    _ => 0,
                }
            } else {
                0
            };

            // Get latest block number for confirmations
            let latest_block = provider.get_latest_block_number().await.unwrap_or(0);
            let confirmations = transaction
                .block_number
                .map(|bn| latest_block.saturating_sub(bn))
                .unwrap_or(0);

            let summary = tx_to_summary(&transaction, receipt.as_ref(), block_timestamp);

            let input_data = transaction.inner.input().clone();

            // Try to decode input data and resolve method name
            let (decoded_input, method_name) = if input_data.len() >= 4 {
                let selector: [u8; 4] = input_data[..4].try_into().unwrap_or([0; 4]);
                let to_address = transaction.inner.to();

                let mut decoded = None;
                let mut mname = None;

                if let Some(to) = to_address {
                    // Try resolving ABI for the target contract
                    let chain_id = provider.chain_id();
                    if let Some(resolved) = abi_resolver.resolve(chain_id, to).await {
                        decoded = TxDecoder::decode_input(&resolved.abi, &input_data);
                        if let Some(ref d) = decoded {
                            mname = Some(d.function_name.clone());
                        }
                    }
                }

                // If no method name from ABI decode, try builtin selectors first (fast, local)
                if mname.is_none() {
                    mname = abi_resolver.match_builtin_selector(selector);
                }

                // If still no name, try 4byte.directory
                if mname.is_none() {
                    if let Some(sig) = abi_resolver.resolve_selector(selector).await {
                        // Extract just the function name from the signature (before the '(')
                        mname = Some(
                            sig.split('(')
                                .next()
                                .unwrap_or(&sig)
                                .to_string(),
                        );
                    }
                }

                (decoded, mname)
            } else {
                (None, None)
            };

            // Extract token transfers from receipt logs
            let token_transfers = receipt
                .as_ref()
                .map(|r| TxDecoder::extract_token_transfers(r.inner.logs()))
                .unwrap_or_default();

            let logs_count = receipt
                .as_ref()
                .map(|r| r.inner.logs().len())
                .unwrap_or(0);

            let mut detail = TransactionDetail {
                summary,
                nonce: transaction.inner.nonce(),
                input_data,
                decoded_input,
                gas_limit: transaction.inner.gas_limit(),
                max_fee_per_gas: Some(transaction.inner.max_fee_per_gas()),
                max_priority_fee_per_gas: transaction.inner.max_priority_fee_per_gas(),

                effective_gas_price: receipt.as_ref().map(|r| r.effective_gas_price),
                token_transfers,
                logs_count,
                confirmations,
            };

            // Set the resolved method name on the summary
            if method_name.is_some() {
                detail.summary.method_name = method_name;
            }

            {
                let mut c = cache.write().await;
                c.put_transaction(hash, detail.clone());
            }

            let _ = tx.send(AppEvent::TransactionDetailLoaded(Box::new(detail)));
        });
    }

    /// Fetch address information: balance, nonce, contract status, proxy detection, and tx history.
    pub fn fetch_address_info(&self, address: Address) {
        let provider = Arc::clone(&self.provider);
        let abi_resolver = Arc::clone(&self.abi_resolver);
        let tx = self.event_tx.clone();
        let etherscan_key = self.etherscan_api_key.clone();

        tokio::spawn(async move {
            // Fetch balance, nonce, and code in parallel
            let (balance_result, nonce_result, is_contract_result) = tokio::join!(
                provider.get_balance(address),
                provider.get_nonce(address),
                provider.is_contract(address),
            );

            let balance = balance_result.unwrap_or(U256::ZERO);
            let nonce = nonce_result.unwrap_or(0);
            let is_contract = is_contract_result.unwrap_or(false);

            // Build contract info if this is a contract
            let contract_info = if is_contract {
                let chain_id = provider.chain_id();
                let resolved = abi_resolver.resolve(chain_id, address).await;

                // EIP-1967 proxy detection
                let (is_proxy, implementation) =
                    match provider.get_storage_at(address, EIP1967_IMPL_SLOT).await {
                        Ok(slot_value) => {
                            if slot_value != U256::ZERO {
                                // Convert U256 to Address (last 20 bytes)
                                let bytes: [u8; 32] = slot_value.to_be_bytes();
                                let impl_addr = Address::from_slice(&bytes[12..]);
                                (true, Some(impl_addr))
                            } else {
                                (false, None)
                            }
                        }
                        Err(_) => (false, None),
                    };

                // If proxy, also resolve the implementation ABI
                if is_proxy {
                    if let Some(impl_addr) = implementation {
                        let _ = abi_resolver.resolve(chain_id, impl_addr).await;
                    }
                }

                Some(ContractInfo {
                    abi_source: resolved.map(|r| r.source),
                    is_proxy,
                    implementation,
                    contract_type: None,
                    name: None,
                    symbol: None,
                    decimals: None,
                })
            } else {
                None
            };

            // Fetch recent transactions from Etherscan if API key available
            let transactions = if let Some(ref api_key) = etherscan_key {
                fetch_etherscan_tx_history(address, api_key).await
            } else {
                vec![]
            };

            let info = AddressInfo {
                address,
                balance,
                nonce,
                is_contract,
                transactions,
                contract_info,
            };

            let _ = tx.send(AppEvent::AddressInfoLoaded(Box::new(info)));
        });
    }

    /// Fetch gas price information from fee history.
    pub fn fetch_gas_info(&self) {
        let provider = Arc::clone(&self.provider);
        let cache = Arc::clone(&self.cache);
        let tx = self.event_tx.clone();

        tokio::spawn(async move {
            // Check cache
            {
                let c = cache.read().await;
                if let Some(cached) = c.get_gas_info() {
                    let _ = tx.send(AppEvent::GasInfoLoaded(cached.clone()));
                    return;
                }
            }

            let fee_history = match provider.get_fee_history(20).await {
                Ok(fh) => fh,
                Err(e) => {
                    let _ =
                        tx.send(AppEvent::Error(format!("Failed to fetch fee history: {e}")));
                    return;
                }
            };

            // base_fee_per_gas contains N+1 entries (one per block + the next predicted)
            let base_fees = &fee_history.base_fee_per_gas;
            let base_fee = base_fees.last().copied().unwrap_or(0);

            // reward contains per-block reward arrays at the requested percentiles.
            // It is Option<Vec<Vec<u128>>>, so unwrap the outer Option first.
            let reward_data = fee_history.reward.as_deref().unwrap_or(&[]);

            // Use the latest block's reward percentiles for current gas estimates
            let (slow, standard, fast) = if let Some(latest_rewards) = reward_data.last() {
                let slow_tip: u128 = latest_rewards.first().copied().unwrap_or(0);
                let standard_tip: u128 = latest_rewards.get(1).copied().unwrap_or(0);
                let fast_tip: u128 = latest_rewards.get(2).copied().unwrap_or(0);
                (
                    base_fee.saturating_add(slow_tip),
                    base_fee.saturating_add(standard_tip),
                    base_fee.saturating_add(fast_tip),
                )
            } else {
                // Fallback to just the base fee
                (base_fee, base_fee, base_fee)
            };

            // Build history from base fees (exclude the predicted next one)
            let history: Vec<u128> = base_fees
                .iter()
                .take(base_fees.len().saturating_sub(1))
                .copied()
                .collect();

            // Build priority fee percentiles from reward data
            let priority_fee_percentiles: Vec<(u8, u128)> =
                if let Some(latest_rewards) = reward_data.last() {
                    [25u8, 50, 75]
                        .iter()
                        .zip(latest_rewards.iter())
                        .map(|(&pct, &val)| (pct, val))
                        .collect()
                } else {
                    vec![]
                };

            // Congestion: base fee above 100 gwei
            let is_congested = base_fee > 100_000_000_000;

            let gas_info = GasInfo {
                slow,
                standard,
                fast,
                base_fee,
                blob_base_fee: fee_history.base_fee_per_blob_gas.last().copied(),
                history,
                priority_fee_percentiles,
                is_congested,
            };

            {
                let mut c = cache.write().await;
                c.put_gas_info(gas_info.clone());
            }

            let _ = tx.send(AppEvent::GasInfoLoaded(gas_info));
        });
    }

    /// Fetch internal transactions (execution trace) for a given transaction.
    pub fn fetch_internal_transactions(&self, tx_hash: B256) {
        let provider = Arc::clone(&self.provider);
        let tx = self.event_tx.clone();

        tokio::spawn(async move {
            // Try trace_transaction first (Parity/Erigon), then debug_traceTransaction (Geth)
            let calls = match fetch_trace_transaction(&provider, tx_hash).await {
                Ok(calls) => calls,
                Err(_) => {
                    // Fallback to debug_traceTransaction with callTracer
                    match fetch_debug_trace(&provider, tx_hash).await {
                        Ok(calls) => calls,
                        Err(e) => {
                            let _ = tx.send(AppEvent::Error(format!(
                                "Failed to trace transaction: {e}"
                            )));
                            return;
                        }
                    }
                }
            };

            let _ = tx.send(AppEvent::InternalTransactionsLoaded {
                tx_hash,
                calls,
            });
        });
    }

    /// Fetch token metadata (name, symbol, decimals) for a single address.
    pub fn fetch_token_metadata(&self, address: Address) {
        let provider = Arc::clone(&self.provider);
        let tx = self.event_tx.clone();

        tokio::spawn(async move {
            let name_data = Bytes::from(IERC20Metadata::nameCall {}.abi_encode());
            let symbol_data = Bytes::from(IERC20Metadata::symbolCall {}.abi_encode());
            let decimals_data = Bytes::from(IERC20Metadata::decimalsCall {}.abi_encode());

            // Try multicall first, fall back to individual calls
            let (name, symbol, decimals) =
                match provider
                    .multicall(vec![
                        (address, name_data.clone()),
                        (address, symbol_data.clone()),
                        (address, decimals_data.clone()),
                    ])
                    .await
                {
                    Ok(results) if results.len() == 3 => {
                        let name = decode_string_result(&results[0]);
                        let symbol = decode_string_result(&results[1]);
                        let decimals = decode_u8_result(&results[2]);
                        (name, symbol, decimals)
                    }
                    _ => {
                        // Fall back to individual calls
                        let name = provider
                            .call(address, name_data)
                            .await
                            .ok()
                            .and_then(|r| decode_string_result(&r));
                        let symbol = provider
                            .call(address, symbol_data)
                            .await
                            .ok()
                            .and_then(|r| decode_string_result(&r));
                        let decimals = provider
                            .call(address, decimals_data)
                            .await
                            .ok()
                            .and_then(|r| decode_u8_result(&r));
                        (name, symbol, decimals)
                    }
                };

            let metadata = TokenMetadata {
                address,
                name: name.unwrap_or_else(|| "Unknown".to_string()),
                symbol: symbol.unwrap_or_else(|| "???".to_string()),
                decimals: decimals.unwrap_or(18),
            };

            let _ = tx.send(AppEvent::TokenMetadataLoaded(metadata));
        });
    }

    /// Parse a search query and fetch the appropriate data, then navigate to the result.
    pub fn search(&self, query: String) {
        let provider = Arc::clone(&self.provider);
        let tx = self.event_tx.clone();

        tokio::spawn(async move {
            let target = match SearchTarget::parse(&query) {
                Some(t) => t,
                None => {
                    let _ = tx.send(AppEvent::SearchNotFound(format!(
                        "Could not parse search query: {query}"
                    )));
                    return;
                }
            };

            match &target {
                SearchTarget::BlockNumber(number) => {
                    // Verify the block exists
                    match provider.get_block(*number).await {
                        Ok(Some(_)) => {
                            let _ = tx.send(AppEvent::SearchResult(target.clone()));
                            let _ = tx.send(AppEvent::Navigate(View::BlockDetail(*number)));
                        }
                        Ok(None) => {
                            let _ = tx.send(AppEvent::SearchNotFound(format!(
                                "Block {number} not found"
                            )));
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::Error(format!("Search error: {e}")));
                        }
                    }
                }
                SearchTarget::TransactionHash(hash) => {
                    // Verify the transaction exists
                    match provider.get_transaction(*hash).await {
                        Ok(Some(_)) => {
                            let _ = tx.send(AppEvent::SearchResult(target.clone()));
                            let _ = tx.send(AppEvent::Navigate(View::TransactionDetail(*hash)));
                        }
                        Ok(None) => {
                            let _ = tx.send(AppEvent::SearchNotFound(format!(
                                "Transaction {hash} not found"
                            )));
                        }
                        Err(_) => {
                            // A 66-char hex could also be a block hash; try that
                            match provider.get_block_by_hash(*hash).await {
                                Ok(Some(block)) => {
                                    let block_num = block.header.number;
                                    let _ = tx.send(AppEvent::SearchResult(
                                        SearchTarget::BlockHash(*hash),
                                    ));
                                    let _ = tx.send(AppEvent::Navigate(View::BlockDetail(
                                        block_num,
                                    )));
                                }
                                _ => {
                                    let _ = tx.send(AppEvent::SearchNotFound(format!(
                                        "No transaction or block found for {hash}"
                                    )));
                                }
                            }
                        }
                    }
                }
                SearchTarget::BlockHash(hash) => {
                    match provider.get_block_by_hash(*hash).await {
                        Ok(Some(block)) => {
                            let block_num = block.header.number;
                            let _ = tx.send(AppEvent::SearchResult(target.clone()));
                            let _ = tx.send(AppEvent::Navigate(View::BlockDetail(block_num)));
                        }
                        Ok(None) => {
                            let _ = tx.send(AppEvent::SearchNotFound(format!(
                                "Block with hash {hash} not found"
                            )));
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::Error(format!("Search error: {e}")));
                        }
                    }
                }
                SearchTarget::Address(address) => {
                    let _ = tx.send(AppEvent::SearchResult(target.clone()));
                    let _ = tx.send(AppEvent::Navigate(View::AddressView(*address)));
                }
                SearchTarget::EnsName(name) => {
                    // Stub - Agent B will implement ENS resolution
                    let _ = tx.send(AppEvent::EnsNotFound(format!(
                        "ENS resolution not yet implemented for: {name}"
                    )));
                }
            }
        });
    }
}

// --- Internal transaction tracing ---

/// Fetch internal calls using Parity-style trace_transaction RPC.
async fn fetch_trace_transaction(
    provider: &EthProvider,
    tx_hash: B256,
) -> color_eyre::eyre::Result<Vec<InternalCall>> {
    let params = serde_json::json!([format!("{tx_hash:?}")]);
    let result = provider.raw_request("trace_transaction", params).await?;

    let traces = result
        .as_array()
        .ok_or_else(|| color_eyre::eyre::eyre!("Expected array from trace_transaction"))?;

    let mut calls = Vec::new();
    for trace in traces {
        let action = &trace["action"];
        let result_field = &trace["result"];

        let from = action["from"]
            .as_str()
            .and_then(|s| s.parse::<Address>().ok())
            .unwrap_or(Address::ZERO);
        let to = action["to"]
            .as_str()
            .and_then(|s| s.parse::<Address>().ok())
            .unwrap_or(Address::ZERO);
        let value = action["value"]
            .as_str()
            .and_then(|s| U256::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .unwrap_or(U256::ZERO);
        let call_type = action["callType"]
            .as_str()
            .unwrap_or("call")
            .to_string();
        let gas_used = result_field["gasUsed"]
            .as_str()
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .unwrap_or(0);
        let input = action["input"]
            .as_str()
            .and_then(|s| {
                let s = s.trim_start_matches("0x");
                alloy::primitives::hex::decode(s).ok()
            })
            .map(Bytes::from)
            .unwrap_or_default();
        let output = result_field["output"]
            .as_str()
            .and_then(|s| {
                let s = s.trim_start_matches("0x");
                alloy::primitives::hex::decode(s).ok()
            })
            .map(Bytes::from)
            .unwrap_or_default();

        let trace_addr = trace["traceAddress"].as_array();
        let depth = trace_addr.map(|a| a.len()).unwrap_or(0);
        let error = trace["error"].as_str().map(|s| s.to_string());

        calls.push(InternalCall {
            from,
            to,
            value,
            call_type,
            gas_used,
            input,
            output,
            depth,
            error,
        });
    }

    Ok(calls)
}

/// Fetch internal calls using Geth-style debug_traceTransaction with callTracer.
async fn fetch_debug_trace(
    provider: &EthProvider,
    tx_hash: B256,
) -> color_eyre::eyre::Result<Vec<InternalCall>> {
    let params = serde_json::json!([
        format!("{tx_hash:?}"),
        {"tracer": "callTracer", "tracerConfig": {"onlyTopCall": false}}
    ]);
    let result = provider
        .raw_request("debug_traceTransaction", params)
        .await?;

    let mut calls = Vec::new();
    parse_call_frame(&result, 0, &mut calls);
    Ok(calls)
}

/// Recursively parse a callTracer frame into flat InternalCall entries.
fn parse_call_frame(frame: &serde_json::Value, depth: usize, calls: &mut Vec<InternalCall>) {
    let from = frame["from"]
        .as_str()
        .and_then(|s| s.parse::<Address>().ok())
        .unwrap_or(Address::ZERO);
    let to = frame["to"]
        .as_str()
        .and_then(|s| s.parse::<Address>().ok())
        .unwrap_or(Address::ZERO);
    let value = frame["value"]
        .as_str()
        .and_then(|s| U256::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        .unwrap_or(U256::ZERO);
    let call_type = frame["type"]
        .as_str()
        .unwrap_or("CALL")
        .to_uppercase();
    let gas_used = frame["gasUsed"]
        .as_str()
        .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        .unwrap_or(0);
    let input = frame["input"]
        .as_str()
        .and_then(|s| {
            let s = s.trim_start_matches("0x");
            alloy::primitives::hex::decode(s).ok()
        })
        .map(Bytes::from)
        .unwrap_or_default();
    let output = frame["output"]
        .as_str()
        .and_then(|s| {
            let s = s.trim_start_matches("0x");
            alloy::primitives::hex::decode(s).ok()
        })
        .map(Bytes::from)
        .unwrap_or_default();
    let error = frame["error"].as_str().map(|s| s.to_string());

    calls.push(InternalCall {
        from,
        to,
        value,
        call_type,
        gas_used,
        input,
        output,
        depth,
        error,
    });

    // Recurse into child calls
    if let Some(sub_calls) = frame["calls"].as_array() {
        for sub in sub_calls {
            parse_call_frame(sub, depth + 1, calls);
        }
    }
}

// --- Token metadata helpers ---

fn decode_string_result(data: &[u8]) -> Option<String> {
    if data.len() < 64 {
        return None;
    }
    // ABI-encoded string: offset (32 bytes) + length (32 bytes) + data
    let offset = U256::from_be_slice(&data[..32]).to::<usize>();
    if offset + 32 > data.len() {
        return None;
    }
    let len = U256::from_be_slice(&data[offset..offset + 32]).to::<usize>();
    let start = offset + 32;
    if start + len > data.len() {
        return None;
    }
    String::from_utf8(data[start..start + len].to_vec()).ok()
}

fn decode_u8_result(data: &[u8]) -> Option<u8> {
    if data.len() < 32 {
        return None;
    }
    Some(data[31])
}

// --- Etherscan tx history ---

async fn fetch_etherscan_tx_history(
    address: Address,
    api_key: &str,
) -> Vec<TransactionSummary> {
    let url = format!(
        "https://api.etherscan.io/api?module=account&action=txlist&address={address}&startblock=0&endblock=99999999&page=1&offset=20&sort=desc&apikey={api_key}"
    );

    let client = reqwest::Client::new();
    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let results = match body["result"].as_array() {
        Some(arr) => arr,
        None => return vec![],
    };

    results
        .iter()
        .filter_map(|item| {
            let hash = item["hash"]
                .as_str()
                .and_then(|s| s.parse::<B256>().ok())?;
            let block_number = item["blockNumber"]
                .as_str()
                .and_then(|s| s.parse::<u64>().ok());
            let timestamp = item["timeStamp"]
                .as_str()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            let from = item["from"]
                .as_str()
                .and_then(|s| s.parse::<Address>().ok())
                .unwrap_or(Address::ZERO);
            let to = item["to"]
                .as_str()
                .and_then(|s| s.parse::<Address>().ok());
            let value = item["value"]
                .as_str()
                .and_then(|s| s.parse::<U256>().ok())
                .unwrap_or(U256::ZERO);
            let gas_used = item["gasUsed"]
                .as_str()
                .and_then(|s| s.parse::<u64>().ok());
            let gas_price = item["gasPrice"]
                .as_str()
                .and_then(|s| s.parse::<u128>().ok());
            let is_error = item["isError"].as_str().unwrap_or("0") == "1";

            let input_str = item["input"].as_str().unwrap_or("0x");
            let method_id = if input_str.len() >= 10 {
                let hex = input_str.trim_start_matches("0x");
                alloy::primitives::hex::decode(&hex[..8])
                    .ok()
                    .and_then(|b| {
                        let arr: [u8; 4] = b.try_into().ok()?;
                        Some(arr)
                    })
            } else {
                None
            };

            Some(TransactionSummary {
                hash,
                block_number,
                timestamp,
                from,
                to,
                value,
                gas_used,
                gas_price,
                method_id,
                method_name: None,
                tx_type: TxType::EIP1559,
                status: if is_error {
                    TxStatus::Failed
                } else {
                    TxStatus::Success
                },
            })
        })
        .collect()
}

// --- Conversion helpers ---

/// Convert an alloy `Block` to our `BlockSummary`.
pub(crate) fn block_to_summary(block: &Block) -> BlockSummary {
    let tx_count = block
        .transactions
        .as_transactions()
        .map(|txs| txs.len())
        .unwrap_or_else(|| {
            block
                .transactions
                .as_hashes()
                .map(|h| h.len())
                .unwrap_or(0)
        });

    let base_fee = block.header.base_fee_per_gas.map(|v| v as u128);
    let gas_used = block.header.gas_used;
    let eth_burned = base_fee.map(|bf| U256::from(bf) * U256::from(gas_used));

    BlockSummary {
        number: block.header.number,
        hash: block.header.hash,
        timestamp: block.header.timestamp,
        tx_count,
        gas_used,
        gas_limit: block.header.gas_limit,
        base_fee,
        miner: block.header.beneficiary,
        eth_burned,
    }
}

/// Convert an alloy `Transaction` (with optional receipt) to our `TransactionSummary`.
pub(crate) fn tx_to_summary(
    tx: &Transaction,
    receipt: Option<&TransactionReceipt>,
    block_timestamp: u64,
) -> TransactionSummary {
    let to = tx.inner.to();
    let is_contract_creation = to.is_none();

    let tx_type_val = tx.inner.tx_type();
    let tx_type = if is_contract_creation {
        TxType::ContractCreation
    } else {
        // alloy::consensus::TxType is an enum, match on variants
        match tx_type_val {
            alloy::consensus::TxType::Legacy => TxType::Legacy,
            alloy::consensus::TxType::Eip2930 => TxType::EIP2930,
            alloy::consensus::TxType::Eip1559 => TxType::EIP1559,
            alloy::consensus::TxType::Eip4844 => TxType::EIP4844,
            _ => TxType::Legacy,
        }
    };

    let status = match receipt {
        Some(r) => {
            if r.status() {
                TxStatus::Success
            } else {
                TxStatus::Failed
            }
        }
        None => TxStatus::Pending,
    };

    let input = tx.inner.input();
    let method_id = if input.len() >= 4 {
        let mut sel = [0u8; 4];
        sel.copy_from_slice(&input[..4]);
        Some(sel)
    } else {
        None
    };

    let gas_used = receipt.map(|r| r.gas_used);

    // Get the sender address from the Recovered wrapper
    let from = tx.inner.signer();

    TransactionSummary {
        hash: *tx.inner.tx_hash(),
        block_number: tx.block_number,
        timestamp: block_timestamp,
        from,
        to,
        value: tx.inner.value(),
        gas_used,
        gas_price: tx.inner.gas_price(),
        method_id,
        method_name: None,
        tx_type,
        status,
    }
}
