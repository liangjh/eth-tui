use alloy::primitives::{Address, Bytes, B256, U256};

#[derive(Debug, Clone)]
pub struct BlockSummary {
    pub number: u64,
    pub hash: B256,
    pub timestamp: u64,
    pub tx_count: usize,
    pub gas_used: u64,
    pub gas_limit: u64,
    pub base_fee: Option<u128>,
    pub miner: Address,
}

#[derive(Debug, Clone)]
pub struct BlockDetail {
    pub summary: BlockSummary,
    pub parent_hash: B256,
    pub state_root: B256,
    pub size: Option<u64>,
    pub transactions: Vec<TransactionSummary>,
    pub total_difficulty: Option<U256>,
}

#[derive(Debug, Clone)]
pub struct TransactionSummary {
    pub hash: B256,
    pub block_number: Option<u64>,
    pub timestamp: u64,
    pub from: Address,
    pub to: Option<Address>,
    pub value: U256,
    pub gas_used: Option<u64>,
    pub gas_price: Option<u128>,
    pub method_id: Option<[u8; 4]>,
    pub method_name: Option<String>,
    pub tx_type: TxType,
    pub status: TxStatus,
}

#[derive(Debug, Clone)]
pub struct TransactionDetail {
    pub summary: TransactionSummary,
    pub nonce: u64,
    pub input_data: Bytes,
    pub decoded_input: Option<DecodedCall>,
    pub gas_limit: u64,
    pub max_fee_per_gas: Option<u128>,
    pub max_priority_fee_per_gas: Option<u128>,
    pub effective_gas_price: Option<u128>,
    pub token_transfers: Vec<TokenTransfer>,
    pub logs_count: usize,
    pub confirmations: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxType {
    Legacy,
    EIP2930,
    EIP1559,
    EIP4844,
    ContractCreation,
}

impl std::fmt::Display for TxType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TxType::Legacy => write!(f, "Legacy (Type 0)"),
            TxType::EIP2930 => write!(f, "EIP-2930 (Type 1)"),
            TxType::EIP1559 => write!(f, "EIP-1559 (Type 2)"),
            TxType::EIP4844 => write!(f, "EIP-4844 (Type 3)"),
            TxType::ContractCreation => write!(f, "Contract Creation"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxStatus {
    Success,
    Failed,
    Pending,
}

impl std::fmt::Display for TxStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TxStatus::Success => write!(f, "Success"),
            TxStatus::Failed => write!(f, "Failed"),
            TxStatus::Pending => write!(f, "Pending"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DecodedCall {
    pub function_name: String,
    pub params: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct TokenTransfer {
    pub token_address: Address,
    pub from: Address,
    pub to: Address,
    pub value: U256,
    pub token_name: Option<String>,
    pub token_symbol: Option<String>,
    pub decimals: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct AddressInfo {
    pub address: Address,
    pub balance: U256,
    pub nonce: u64,
    pub is_contract: bool,
    pub transactions: Vec<TransactionSummary>,
    pub contract_info: Option<ContractInfo>,
}

#[derive(Debug, Clone)]
pub struct ContractInfo {
    pub abi_source: Option<String>,
    pub is_proxy: bool,
    pub implementation: Option<Address>,
    pub contract_type: Option<ContractType>,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub decimals: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractType {
    ERC20,
    ERC721,
    ERC1155,
    Unknown,
}

impl std::fmt::Display for ContractType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractType::ERC20 => write!(f, "ERC-20"),
            ContractType::ERC721 => write!(f, "ERC-721"),
            ContractType::ERC1155 => write!(f, "ERC-1155"),
            ContractType::Unknown => write!(f, "Contract"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tx_type_display() {
        assert_eq!(TxType::Legacy.to_string(), "Legacy (Type 0)");
        assert_eq!(TxType::EIP2930.to_string(), "EIP-2930 (Type 1)");
        assert_eq!(TxType::EIP1559.to_string(), "EIP-1559 (Type 2)");
        assert_eq!(TxType::EIP4844.to_string(), "EIP-4844 (Type 3)");
        assert_eq!(TxType::ContractCreation.to_string(), "Contract Creation");
    }

    #[test]
    fn test_tx_status_display() {
        assert_eq!(TxStatus::Success.to_string(), "Success");
        assert_eq!(TxStatus::Failed.to_string(), "Failed");
        assert_eq!(TxStatus::Pending.to_string(), "Pending");
    }

    #[test]
    fn test_contract_type_display() {
        assert_eq!(ContractType::ERC20.to_string(), "ERC-20");
        assert_eq!(ContractType::ERC721.to_string(), "ERC-721");
        assert_eq!(ContractType::ERC1155.to_string(), "ERC-1155");
        assert_eq!(ContractType::Unknown.to_string(), "Contract");
    }
}

#[derive(Debug, Clone)]
pub struct GasInfo {
    pub slow: u128,
    pub standard: u128,
    pub fast: u128,
    pub base_fee: u128,
    pub blob_base_fee: Option<u128>,
    pub history: Vec<u128>,
}
