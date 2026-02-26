use alloy::primitives::{Address, B256, U256};

use crate::data::types::{
    AddressInfo, BlockDetail, BlockSummary, DecodedLog, ExecutionTrace, GasInfo, InternalCall,
    TokenMetadata, TransactionDetail, TransactionSummary, WatchEntry,
};

/// Views the user can navigate to
#[derive(Debug, Clone)]
pub enum View {
    Dashboard,
    BlockList,
    BlockDetail(u64),
    TransactionDetail(B256),
    AddressView(Address),
    GasTracker,
    WatchList,
    Mempool,
    TxDebugger(B256),
    ContractRead(Address),
    StorageInspector(Address),
}

/// Target identified from a search query
#[derive(Debug, Clone)]
pub enum SearchTarget {
    Address(Address),
    TransactionHash(B256),
    BlockNumber(u64),
    BlockHash(B256),
    EnsName(String),
}

impl SearchTarget {
    pub fn parse(input: &str) -> Option<SearchTarget> {
        let input = input.trim();

        // ENS name (ends with .eth)
        if input.ends_with(".eth") && input.len() > 4 {
            return Some(SearchTarget::EnsName(input.to_string()));
        }

        // 0x-prefixed, 66 chars = tx hash or block hash
        if input.starts_with("0x") && input.len() == 66 {
            if let Ok(hash) = input.parse::<B256>() {
                return Some(SearchTarget::TransactionHash(hash));
            }
        }

        // 0x-prefixed, 42 chars = address
        if input.starts_with("0x") && input.len() == 42 {
            if let Ok(addr) = input.parse::<Address>() {
                return Some(SearchTarget::Address(addr));
            }
        }

        // Pure number = block number
        if let Ok(num) = input.parse::<u64>() {
            return Some(SearchTarget::BlockNumber(num));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_address() {
        let input = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
        let result = SearchTarget::parse(input);
        assert!(matches!(result, Some(SearchTarget::Address(_))));
    }

    #[test]
    fn test_parse_tx_hash() {
        let input = "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let result = SearchTarget::parse(input);
        assert!(matches!(result, Some(SearchTarget::TransactionHash(_))));
    }

    #[test]
    fn test_parse_block_number() {
        let result = SearchTarget::parse("19234567");
        assert!(matches!(result, Some(SearchTarget::BlockNumber(19234567))));
    }

    #[test]
    fn test_parse_block_zero() {
        let result = SearchTarget::parse("0");
        assert!(matches!(result, Some(SearchTarget::BlockNumber(0))));
    }

    #[test]
    fn test_parse_empty() {
        assert!(SearchTarget::parse("").is_none());
    }

    #[test]
    fn test_parse_invalid() {
        assert!(SearchTarget::parse("hello world").is_none());
        assert!(SearchTarget::parse("0x").is_none());
        assert!(SearchTarget::parse("0xZZZZ").is_none());
    }

    #[test]
    fn test_parse_whitespace_trimming() {
        let result = SearchTarget::parse("  19234567  ");
        assert!(matches!(result, Some(SearchTarget::BlockNumber(19234567))));
    }

    #[test]
    fn test_parse_short_hex_not_address() {
        // 0x-prefixed but not 42 chars and not 66 chars
        assert!(SearchTarget::parse("0xabcdef").is_none());
    }
}

/// Events sent from background data tasks to the main app loop
#[derive(Debug)]
pub enum AppEvent {
    // Data loaded
    LatestBlockNumber(u64),
    RecentBlocks(Vec<BlockSummary>),
    RecentTransactions(Vec<TransactionSummary>),
    BlockDetailLoaded(Box<BlockDetail>),
    TransactionDetailLoaded(Box<TransactionDetail>),
    AddressInfoLoaded(Box<AddressInfo>),
    GasInfoLoaded(GasInfo),

    // ENS
    EnsResolved { name: String, address: Address },
    EnsNotFound(String),

    // Token metadata
    TokenMetadataLoaded(TokenMetadata),

    // Internal transactions
    InternalTransactionsLoaded { tx_hash: B256, calls: Vec<InternalCall> },

    // Event logs decoded
    DecodedLogsLoaded { tx_hash: B256, logs: Vec<DecodedLog> },

    // Contract read
    ContractReadResult { address: Address, function: String, result: String },

    // Watch list
    WatchListUpdated(Vec<WatchEntry>),

    // Mempool / WebSocket
    PendingTransactions(Vec<TransactionSummary>),
    WsConnected,
    WsDisconnected,
    NewBlock(BlockSummary),
    NewPendingTx(TransactionSummary),

    // Tx debugger
    TraceLoaded { tx_hash: B256, trace: ExecutionTrace },

    // Storage
    StorageValueLoaded { address: Address, slot: U256, value: B256 },

    // Export
    ExportComplete(String),

    // Search
    SearchResult(SearchTarget),
    SearchNotFound(String),

    // Navigation
    Navigate(View),
    Back,

    // Status
    Error(String),
    Connected(u64), // chain_id
}
