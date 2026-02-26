use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

use alloy::primitives::{Address, B256, U256};
use lru::LruCache;

use crate::data::types::*;

/// TTL durations for cached data categories.
const BLOCK_TTL: Duration = Duration::from_secs(3600); // blocks are immutable, long TTL
const TX_TTL: Duration = Duration::from_secs(3600); // transactions are immutable
const BALANCE_TTL: Duration = Duration::from_secs(30); // balances change often
const GAS_TTL: Duration = Duration::from_secs(12); // roughly one block
const TOKEN_METADATA_TTL: Duration = Duration::from_secs(3600); // token metadata rarely changes

/// Cache sizes for each data type.
const BLOCK_CACHE_SIZE: usize = 500;
const BLOCK_DETAIL_CACHE_SIZE: usize = 100;
const TX_CACHE_SIZE: usize = 500;
const BALANCE_CACHE_SIZE: usize = 200;
const TOKEN_METADATA_CACHE_SIZE: usize = 500;

pub struct DataCache {
    blocks: LruCache<u64, (Instant, BlockSummary)>,
    block_details: LruCache<u64, (Instant, BlockDetail)>,
    transactions: LruCache<B256, (Instant, TransactionDetail)>,
    balances: LruCache<Address, (Instant, U256)>,
    gas_info: Option<(Instant, GasInfo)>,
    token_metadata: LruCache<Address, (Instant, TokenMetadata)>,
}

impl DataCache {
    pub fn new() -> Self {
        Self {
            blocks: LruCache::new(NonZeroUsize::new(BLOCK_CACHE_SIZE).unwrap()),
            block_details: LruCache::new(NonZeroUsize::new(BLOCK_DETAIL_CACHE_SIZE).unwrap()),
            transactions: LruCache::new(NonZeroUsize::new(TX_CACHE_SIZE).unwrap()),
            balances: LruCache::new(NonZeroUsize::new(BALANCE_CACHE_SIZE).unwrap()),
            gas_info: None,
            token_metadata: LruCache::new(NonZeroUsize::new(TOKEN_METADATA_CACHE_SIZE).unwrap()),
        }
    }

    // --- Block Summary ---

    /// Get a cached block summary, returning a clone. Returns None if expired or missing.
    pub fn get_block(&mut self, number: u64) -> Option<BlockSummary> {
        let entry = self.blocks.get(&number)?;
        if entry.0.elapsed() < BLOCK_TTL {
            Some(entry.1.clone())
        } else {
            self.blocks.pop(&number);
            None
        }
    }

    pub fn put_block(&mut self, number: u64, block: BlockSummary) {
        self.blocks.put(number, (Instant::now(), block));
    }

    // --- Block Detail ---

    /// Get a cached block detail, returning a clone. Returns None if expired or missing.
    pub fn get_block_detail(&mut self, number: u64) -> Option<BlockDetail> {
        let entry = self.block_details.get(&number)?;
        if entry.0.elapsed() < BLOCK_TTL {
            Some(entry.1.clone())
        } else {
            self.block_details.pop(&number);
            None
        }
    }

    pub fn put_block_detail(&mut self, number: u64, detail: BlockDetail) {
        self.block_details.put(number, (Instant::now(), detail));
    }

    // --- Transaction Detail ---

    /// Get a cached transaction detail, returning a clone. Returns None if expired or missing.
    pub fn get_transaction(&mut self, hash: B256) -> Option<TransactionDetail> {
        let entry = self.transactions.get(&hash)?;
        if entry.0.elapsed() < TX_TTL {
            Some(entry.1.clone())
        } else {
            self.transactions.pop(&hash);
            None
        }
    }

    pub fn put_transaction(&mut self, hash: B256, detail: TransactionDetail) {
        self.transactions.put(hash, (Instant::now(), detail));
    }

    // --- Balance ---

    pub fn get_balance(&mut self, address: Address) -> Option<U256> {
        let entry = self.balances.get(&address)?;
        if entry.0.elapsed() < BALANCE_TTL {
            Some(entry.1)
        } else {
            self.balances.pop(&address);
            None
        }
    }

    pub fn put_balance(&mut self, address: Address, balance: U256) {
        self.balances.put(address, (Instant::now(), balance));
    }

    // --- Gas Info ---

    pub fn get_gas_info(&self) -> Option<&GasInfo> {
        let (instant, info) = self.gas_info.as_ref()?;
        if instant.elapsed() < GAS_TTL {
            Some(info)
        } else {
            None
        }
    }

    pub fn put_gas_info(&mut self, info: GasInfo) {
        self.gas_info = Some((Instant::now(), info));
    }

    // --- Token Metadata ---

    /// Get cached token metadata. Returns None if expired or missing.
    pub fn get_token_metadata(&mut self, address: Address) -> Option<TokenMetadata> {
        let entry = self.token_metadata.get(&address)?;
        if entry.0.elapsed() < TOKEN_METADATA_TTL {
            Some(entry.1.clone())
        } else {
            self.token_metadata.pop(&address);
            None
        }
    }

    /// Cache token metadata with automatic TTL.
    pub fn put_token_metadata(&mut self, address: Address, metadata: TokenMetadata) {
        self.token_metadata.put(address, (Instant::now(), metadata));
    }

    /// Evict all cached data. Useful when switching chains or reconnecting.
    pub fn clear(&mut self) {
        self.blocks.clear();
        self.block_details.clear();
        self.transactions.clear();
        self.balances.clear();
        self.gas_info = None;
        self.token_metadata.clear();
    }
}

impl Default for DataCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_block_summary(number: u64) -> BlockSummary {
        BlockSummary {
            number,
            hash: B256::ZERO,
            timestamp: 1700000000,
            tx_count: 100,
            gas_used: 15_000_000,
            gas_limit: 30_000_000,
            base_fee: Some(30_000_000_000),
            miner: Address::ZERO,
            eth_burned: None,
        }
    }

    fn make_gas_info() -> GasInfo {
        GasInfo {
            slow: 10_000_000_000,
            standard: 20_000_000_000,
            fast: 40_000_000_000,
            base_fee: 15_000_000_000,
            blob_base_fee: None,
            history: vec![10, 20, 30],
            priority_fee_percentiles: vec![],
            is_congested: false,
        }
    }

    fn make_token_metadata(address: Address) -> TokenMetadata {
        TokenMetadata {
            address,
            name: "Test Token".to_string(),
            symbol: "TST".to_string(),
            decimals: 18,
        }
    }

    #[test]
    fn test_put_and_get_block() {
        let mut cache = DataCache::new();
        let block = make_block_summary(100);
        cache.put_block(100, block.clone());

        let cached = cache.get_block(100);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().number, 100);
    }

    #[test]
    fn test_get_missing_block() {
        let mut cache = DataCache::new();
        assert!(cache.get_block(999).is_none());
    }

    #[test]
    fn test_put_and_get_balance() {
        let mut cache = DataCache::new();
        let addr = Address::from_slice(&[0x01; 20]);
        let balance = U256::from(1_000_000u64);

        cache.put_balance(addr, balance);
        let cached = cache.get_balance(addr);
        assert_eq!(cached, Some(balance));
    }

    #[test]
    fn test_get_missing_balance() {
        let mut cache = DataCache::new();
        let addr = Address::from_slice(&[0x99; 20]);
        assert!(cache.get_balance(addr).is_none());
    }

    #[test]
    fn test_put_and_get_gas_info() {
        let mut cache = DataCache::new();
        let gas = make_gas_info();
        cache.put_gas_info(gas);

        let cached = cache.get_gas_info();
        assert!(cached.is_some());
        let info = cached.unwrap();
        assert_eq!(info.slow, 10_000_000_000);
        assert_eq!(info.standard, 20_000_000_000);
        assert_eq!(info.fast, 40_000_000_000);
    }

    #[test]
    fn test_gas_info_initially_none() {
        let cache = DataCache::new();
        assert!(cache.get_gas_info().is_none());
    }

    #[test]
    fn test_put_and_get_token_metadata() {
        let mut cache = DataCache::new();
        let addr = Address::from_slice(&[0x01; 20]);
        let metadata = make_token_metadata(addr);

        cache.put_token_metadata(addr, metadata.clone());
        let cached = cache.get_token_metadata(addr);
        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached.name, "Test Token");
        assert_eq!(cached.symbol, "TST");
        assert_eq!(cached.decimals, 18);
    }

    #[test]
    fn test_get_missing_token_metadata() {
        let mut cache = DataCache::new();
        let addr = Address::from_slice(&[0x99; 20]);
        assert!(cache.get_token_metadata(addr).is_none());
    }

    #[test]
    fn test_clear_empties_all_caches() {
        let mut cache = DataCache::new();
        cache.put_block(1, make_block_summary(1));
        cache.put_balance(Address::ZERO, U256::from(100u64));
        cache.put_gas_info(make_gas_info());
        cache.put_token_metadata(Address::ZERO, make_token_metadata(Address::ZERO));

        cache.clear();

        assert!(cache.get_block(1).is_none());
        assert!(cache.get_balance(Address::ZERO).is_none());
        assert!(cache.get_gas_info().is_none());
        assert!(cache.get_token_metadata(Address::ZERO).is_none());
    }

    #[test]
    fn test_lru_eviction() {
        // Create a cache and fill blocks beyond capacity to test LRU eviction
        let mut cache = DataCache::new();
        // BLOCK_CACHE_SIZE is 500, fill 501
        for i in 0..=BLOCK_CACHE_SIZE as u64 {
            cache.put_block(i, make_block_summary(i));
        }
        // Block 0 should have been evicted (it was the least recently used)
        assert!(cache.get_block(0).is_none());
        // Most recent block should still be present
        assert!(cache.get_block(BLOCK_CACHE_SIZE as u64).is_some());
    }

    #[test]
    fn test_overwrite_existing_key() {
        let mut cache = DataCache::new();
        cache.put_block(1, make_block_summary(1));

        let mut updated = make_block_summary(1);
        updated.tx_count = 999;
        cache.put_block(1, updated);

        let cached = cache.get_block(1).unwrap();
        assert_eq!(cached.tx_count, 999);
    }

    #[test]
    fn test_default_trait() {
        let cache = DataCache::default();
        assert!(cache.gas_info.is_none());
    }
}
