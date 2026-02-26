use std::sync::Arc;
use std::time::Duration;

use alloy::primitives::{Address, U256};
use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use futures::StreamExt;
use tokio::sync::mpsc;

use crate::data::types::{BlockSummary, TransactionSummary, TxStatus, TxType};
use crate::events::AppEvent;

/// WebSocket subscription service for live block and pending transaction events.
pub struct WsService {
    pub event_tx: mpsc::UnboundedSender<AppEvent>,
    shutdown_tx: Option<mpsc::UnboundedSender<()>>,
}

impl WsService {
    pub fn new(event_tx: mpsc::UnboundedSender<AppEvent>) -> Self {
        Self {
            event_tx,
            shutdown_tx: None,
        }
    }

    /// Connect to a WebSocket endpoint and start subscriptions.
    /// Spawns background tasks for newHeads and newPendingTransactions.
    pub async fn connect(&mut self, ws_url: &str) {
        let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel::<()>();
        self.shutdown_tx = Some(shutdown_tx);

        let url = ws_url.to_string();
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            let mut backoff = Duration::from_secs(1);
            let max_backoff = Duration::from_secs(30);

            loop {
                match Self::connect_and_subscribe(&url, event_tx.clone(), &mut shutdown_rx).await {
                    Ok(()) => {
                        // Clean shutdown requested
                        let _ = event_tx.send(AppEvent::WsDisconnected);
                        return;
                    }
                    Err(_) => {
                        let _ = event_tx.send(AppEvent::WsDisconnected);
                        // Exponential backoff reconnection
                        tokio::select! {
                            _ = tokio::time::sleep(backoff) => {
                                backoff = (backoff * 2).min(max_backoff);
                            }
                            _ = shutdown_rx.recv() => {
                                return;
                            }
                        }
                    }
                }
            }
        });
    }

    async fn connect_and_subscribe(
        url: &str,
        event_tx: mpsc::UnboundedSender<AppEvent>,
        shutdown_rx: &mut mpsc::UnboundedReceiver<()>,
    ) -> Result<(), color_eyre::eyre::Report> {
        let ws = WsConnect::new(url.to_string());
        let provider = ProviderBuilder::new().on_ws(ws).await?;
        let provider = Arc::new(provider);

        let _ = event_tx.send(AppEvent::WsConnected);

        // Subscribe to new block headers
        let head_sub = provider.subscribe_blocks().await?;
        let mut head_stream = head_sub.into_stream();

        // Subscribe to pending transactions (full tx objects)
        let pending_sub = provider.subscribe_full_pending_transactions().await?;
        let mut pending_stream = pending_sub.into_stream();

        loop {
            tokio::select! {
                Some(header) = head_stream.next() => {
                    // header is alloy::rpc::types::Header with fields:
                    // hash, inner (consensus Header), total_difficulty, size
                    // The inner consensus header has: number, timestamp, gas_used,
                    // gas_limit, base_fee_per_gas, beneficiary, etc.
                    let base_fee = header.inner.base_fee_per_gas.map(|v| v as u128);
                    let gas_used = header.inner.gas_used;
                    let eth_burned = base_fee.map(|bf| U256::from(bf) * U256::from(gas_used));

                    let summary = BlockSummary {
                        number: header.inner.number,
                        hash: header.hash,
                        timestamp: header.inner.timestamp,
                        tx_count: 0, // Headers don't include transaction count
                        gas_used,
                        gas_limit: header.inner.gas_limit,
                        base_fee,
                        miner: header.inner.beneficiary,
                        eth_burned,
                    };

                    let _ = event_tx.send(AppEvent::NewBlock(summary));
                }
                Some(tx) = pending_stream.next() => {
                    use alloy::consensus::Transaction as ConsensusTx;

                    let input = tx.inner.input();
                    let method_id = if input.len() >= 4 {
                        let mut sel = [0u8; 4];
                        sel.copy_from_slice(&input[..4]);
                        Some(sel)
                    } else {
                        None
                    };

                    let summary = TransactionSummary {
                        hash: *tx.inner.tx_hash(),
                        block_number: None,
                        timestamp: 0,
                        from: tx.inner.signer(),
                        to: tx.inner.to(),
                        value: tx.inner.value(),
                        gas_used: None,
                        gas_price: tx.inner.gas_price(),
                        method_id,
                        method_name: None,
                        tx_type: TxType::EIP1559,
                        status: TxStatus::Pending,
                    };

                    let _ = event_tx.send(AppEvent::NewPendingTx(summary));
                }
                _ = shutdown_rx.recv() => {
                    return Ok(());
                }
            }
        }
    }

    /// Shut down the WebSocket connection.
    pub fn disconnect(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

impl Drop for WsService {
    fn drop(&mut self) {
        self.disconnect();
    }
}
