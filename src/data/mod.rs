pub mod abi;
pub mod cache;
pub mod decoder;
pub mod provider;
pub mod types;

use std::sync::Arc;

use alloy::consensus::Transaction as ConsensusTransaction;
use alloy::primitives::{Address, B256, U256};
use alloy::rpc::types::{Block, Transaction, TransactionReceipt};
use tokio::sync::mpsc;
use tokio::sync::RwLock;

use crate::data::abi::AbiResolver;
use crate::data::cache::DataCache;
use crate::data::decoder::TxDecoder;
use crate::data::provider::EthProvider;
use crate::data::types::*;
use crate::events::{AppEvent, SearchTarget, View};

pub struct DataService {
    provider: Arc<EthProvider>,
    cache: Arc<RwLock<DataCache>>,
    abi_resolver: Arc<AbiResolver>,
    event_tx: mpsc::UnboundedSender<AppEvent>,
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
            abi_resolver: Arc::new(AbiResolver::new(etherscan_api_key)),
            event_tx,
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

            // Try to decode input data
            let decoded_input = if input_data.len() >= 4 {
                let to_address = transaction.inner.to();
                if let Some(to) = to_address {
                    // Try resolving ABI for the target contract
                    let chain_id = provider.chain_id();
                    if let Some(resolved) = abi_resolver.resolve(chain_id, to).await {
                        TxDecoder::decode_input(&resolved.abi, &input_data)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
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

            let detail = TransactionDetail {
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

            {
                let mut c = cache.write().await;
                c.put_transaction(hash, detail.clone());
            }

            let _ = tx.send(AppEvent::TransactionDetailLoaded(Box::new(detail)));
        });
    }

    /// Fetch address information: balance, nonce, contract status, and recent transactions.
    pub fn fetch_address_info(&self, address: Address) {
        let provider = Arc::clone(&self.provider);
        let abi_resolver = Arc::clone(&self.abi_resolver);
        let tx = self.event_tx.clone();

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
                Some(ContractInfo {
                    abi_source: resolved.map(|r| r.source),
                    is_proxy: false,
                    implementation: None,
                    contract_type: None,
                    name: None,
                    symbol: None,
                    decimals: None,
                })
            } else {
                None
            };

            let info = AddressInfo {
                address,
                balance,
                nonce,
                is_contract,
                transactions: vec![], // Recent txs would require indexing or trace APIs
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

            let gas_info = GasInfo {
                slow,
                standard,
                fast,
                base_fee,
                blob_base_fee: fee_history.base_fee_per_blob_gas.last().copied(),
                history,
            };

            {
                let mut c = cache.write().await;
                c.put_gas_info(gas_info.clone());
            }

            let _ = tx.send(AppEvent::GasInfoLoaded(gas_info));
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
            }
        });
    }
}

// --- Conversion helpers ---

/// Convert an alloy `Block` to our `BlockSummary`.
fn block_to_summary(block: &Block) -> BlockSummary {
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

    BlockSummary {
        number: block.header.number,
        hash: block.header.hash,
        timestamp: block.header.timestamp,
        tx_count,
        gas_used: block.header.gas_used,
        gas_limit: block.header.gas_limit,
        base_fee: block.header.base_fee_per_gas.map(|v| v as u128),
        miner: block.header.beneficiary,
    }
}

/// Convert an alloy `Transaction` (with optional receipt) to our `TransactionSummary`.
fn tx_to_summary(
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
