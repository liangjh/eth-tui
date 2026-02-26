use alloy::eips::BlockId;
use alloy::primitives::{Address, Bytes, B256, U256};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::{Block, BlockNumberOrTag, TransactionReceipt};
use alloy::sol;
use alloy::sol_types::SolCall;
use color_eyre::eyre::Result;

// Multicall3 ABI via sol! macro
sol! {
    #[derive(Debug)]
    interface IMulticall3 {
        struct Call3 {
            address target;
            bool allowFailure;
            bytes callData;
        }
        struct Result {
            bool success;
            bytes returnData;
        }
        function aggregate3(Call3[] calldata calls) external payable returns (Result[] memory returnData);
    }
}

/// Multicall3 deployed address (same on all major chains)
const MULTICALL3_ADDRESS: Address = {
    Address::new([
        0xca, 0x11, 0xbd, 0xe0, 0x59, 0x77, 0xb3, 0x63, 0x11, 0x67, 0x02, 0x88, 0x62, 0xbE,
        0x2a, 0x17, 0x39, 0x76, 0xCA, 0x11,
    ])
};

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

    /// Execute a raw JSON-RPC request (for trace/debug RPCs).
    /// Uses raw_request_dyn which works on trait objects (Box<dyn Provider>).
    pub async fn raw_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let params_str = serde_json::to_string(&params)?;
        let raw_params = serde_json::value::RawValue::from_string(params_str)?;
        let raw_result = self
            .provider
            .raw_request_dyn(method.to_string().into(), &raw_params)
            .await?;
        let result: serde_json::Value = serde_json::from_str(raw_result.get())?;
        Ok(result)
    }

    /// Execute an eth_call (read-only call to a contract).
    pub async fn call(&self, to: Address, data: Bytes) -> Result<Bytes> {
        let tx = alloy::rpc::types::TransactionRequest::default()
            .to(to)
            .input(alloy::rpc::types::TransactionInput::new(data));
        let result = self.provider.call(tx).await?;
        Ok(result)
    }

    /// Batch multiple calls via Multicall3.aggregate3.
    /// Each call is (target_address, calldata). Returns the raw return bytes per call.
    pub async fn multicall(&self, calls: Vec<(Address, Bytes)>) -> Result<Vec<Bytes>> {
        let mc_calls: Vec<IMulticall3::Call3> = calls
            .into_iter()
            .map(|(target, call_data)| IMulticall3::Call3 {
                target,
                allowFailure: true,
                callData: call_data,
            })
            .collect();

        let encoded =
            Bytes::from(IMulticall3::aggregate3Call { calls: mc_calls }.abi_encode());

        let result_bytes = self.call(MULTICALL3_ADDRESS, encoded).await?;

        let decoded = IMulticall3::aggregate3Call::abi_decode_returns(&result_bytes, false)?;
        let results: Vec<Bytes> = decoded
            .returnData
            .into_iter()
            .map(|r| {
                if r.success {
                    Bytes::from(r.returnData.to_vec())
                } else {
                    Bytes::new()
                }
            })
            .collect();

        Ok(results)
    }
}
