use alloy::eips::BlockId;
use alloy::primitives::{Address, Bytes, B256, U256};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::{Block, BlockNumberOrTag, TransactionReceipt};
use color_eyre::eyre::Result;

/// The concrete provider type returned by `ProviderBuilder::new().on_http(url)`.
/// We use a trait-object-based wrapper to avoid spelling out the full generic type.
pub struct EthProvider {
    provider: Box<dyn Provider + Send + Sync>,
    chain_id: u64,
}

impl EthProvider {
    /// Connect to an Ethereum node via HTTP RPC.
    pub async fn connect(rpc_url: &str) -> Result<Self> {
        let url = rpc_url.parse()?;
        let provider = ProviderBuilder::new().on_http(url);
        let chain_id = provider.get_chain_id().await?;
        Ok(Self {
            provider: Box::new(provider),
            chain_id,
        })
    }

    /// Return the chain ID obtained at connection time.
    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    /// Get the latest block number.
    pub async fn get_latest_block_number(&self) -> Result<u64> {
        let number = self.provider.get_block_number().await?;
        Ok(number)
    }

    /// Get a block by number with full transaction objects.
    pub async fn get_block(&self, number: u64) -> Result<Option<Block>> {
        let block = self
            .provider
            .get_block_by_number(BlockNumberOrTag::Number(number))
            .full()
            .await?;
        Ok(block)
    }

    /// Get a block by its hash with full transaction objects.
    pub async fn get_block_by_hash(&self, hash: B256) -> Result<Option<Block>> {
        let block = self
            .provider
            .get_block_by_hash(hash)
            .full()
            .await?;
        Ok(block)
    }

    /// Get a transaction by its hash.
    pub async fn get_transaction(
        &self,
        hash: B256,
    ) -> Result<Option<alloy::rpc::types::Transaction>> {
        let tx = self.provider.get_transaction_by_hash(hash).await?;
        Ok(tx)
    }

    /// Get a transaction receipt by transaction hash.
    pub async fn get_transaction_receipt(&self, hash: B256) -> Result<Option<TransactionReceipt>> {
        let receipt = self.provider.get_transaction_receipt(hash).await?;
        Ok(receipt)
    }

    /// Get the ETH balance of an address at the latest block.
    pub async fn get_balance(&self, address: Address) -> Result<U256> {
        let balance = self.provider.get_balance(address).await?;
        Ok(balance)
    }

    /// Get the deployed bytecode at an address.
    pub async fn get_code(&self, address: Address) -> Result<Bytes> {
        let code = self.provider.get_code_at(address).await?;
        Ok(code)
    }

    /// Get the transaction count (nonce) for an address.
    pub async fn get_nonce(&self, address: Address) -> Result<u64> {
        let nonce = self.provider.get_transaction_count(address).await?;
        Ok(nonce)
    }

    /// Get the current gas price in wei.
    pub async fn get_gas_price(&self) -> Result<u128> {
        let price = self.provider.get_gas_price().await?;
        Ok(price)
    }

    /// Get fee history for the last `block_count` blocks.
    /// Returns base fees and reward percentiles (25th, 50th, 75th).
    pub async fn get_fee_history(
        &self,
        block_count: u64,
    ) -> Result<alloy::rpc::types::FeeHistory> {
        let fee_history = self
            .provider
            .get_fee_history(
                block_count,
                BlockNumberOrTag::Latest,
                &[25.0, 50.0, 75.0],
            )
            .await?;
        Ok(fee_history)
    }

    /// Get all transaction receipts for a given block.
    pub async fn get_block_receipts(&self, number: u64) -> Result<Vec<TransactionReceipt>> {
        let receipts: Option<Vec<TransactionReceipt>> = self
            .provider
            .get_block_receipts(BlockId::Number(BlockNumberOrTag::Number(number)))
            .await?;
        Ok(receipts.unwrap_or_default())
    }

    /// Check whether an address has deployed code (i.e., is a contract).
    pub async fn is_contract(&self, address: Address) -> Result<bool> {
        let code = self.get_code(address).await?;
        Ok(!code.is_empty())
    }

    /// Read a storage slot from a contract.
    pub async fn get_storage_at(&self, address: Address, slot: U256) -> Result<U256> {
        let value = self.provider.get_storage_at(address, slot).await?;
        Ok(value)
    }
}
