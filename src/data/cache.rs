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

/// Cache sizes for each data type.
const BLOCK_CACHE_SIZE: usize = 500;
const BLOCK_DETAIL_CACHE_SIZE: usize = 100;
const TX_CACHE_SIZE: usize = 500;
const BALANCE_CACHE_SIZE: usize = 200;

pub struct DataCache {
    blocks: LruCache<u64, (Instant, BlockSummary)>,
    block_details: LruCache<u64, (Instant, BlockDetail)>,
    transactions: LruCache<B256, (Instant, TransactionDetail)>,
    balances: LruCache<Address, (Instant, U256)>,
    gas_info: Option<(Instant, GasInfo)>,
}

impl DataCache {
    pub fn new() -> Self {
        Self {
            blocks: LruCache::new(NonZeroUsize::new(BLOCK_CACHE_SIZE).unwrap()),
            block_details: LruCache::new(NonZeroUsize::new(BLOCK_DETAIL_CACHE_SIZE).unwrap()),
            transactions: LruCache::new(NonZeroUsize::new(TX_CACHE_SIZE).unwrap()),
            balances: LruCache::new(NonZeroUsize::new(BALANCE_CACHE_SIZE).unwrap()),
            gas_info: None,
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

    /// Evict all cached data. Useful when switching chains or reconnecting.
    pub fn clear(&mut self) {
        self.blocks.clear();
        self.block_details.clear();
        self.transactions.clear();
        self.balances.clear();
        self.gas_info = None;
    }
}

impl Default for DataCache {
    fn default() -> Self {
        Self::new()
    }
}
