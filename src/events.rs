use alloy::primitives::{Address, B256};

use crate::data::types::{
    AddressInfo, BlockDetail, BlockSummary, GasInfo, TransactionDetail, TransactionSummary,
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
}

/// Target identified from a search query
#[derive(Debug, Clone)]
pub enum SearchTarget {
    Address(Address),
    TransactionHash(B256),
    BlockNumber(u64),
    BlockHash(B256),
}

impl SearchTarget {
    pub fn parse(input: &str) -> Option<SearchTarget> {
        let input = input.trim();

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
