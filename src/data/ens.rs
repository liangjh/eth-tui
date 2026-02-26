use alloy::primitives::{address, Address, Bytes, B256, FixedBytes};
use alloy::providers::Provider;
use alloy::rpc::types::TransactionRequest;
use alloy::sol;
use alloy::sol_types::SolCall;

/// ENS registry address on Ethereum mainnet.
const ENS_REGISTRY: Address = address!("00000000000C2E074eC69A0dFb2997BA6C7d2e1e");

// ABI definitions for ENS registry and public resolver
sol! {
    #[allow(missing_docs)]
    function resolver(bytes32 node) external view returns (address);
    #[allow(missing_docs)]
    function addr(bytes32 node) external view returns (address);
}

/// ENS name resolver using manual contract calls (alloy 0.12 has no built-in ENS).
pub struct EnsResolver;

impl EnsResolver {
    pub fn new() -> Self {
        Self
    }

    /// Resolve an ENS name to an Ethereum address.
    ///
    /// 1. Compute the namehash per EIP-137.
    /// 2. Call the ENS registry's `resolver(bytes32)` to find the resolver contract.
    /// 3. Call the resolver's `addr(bytes32)` to get the address.
    pub async fn resolve(
        &self,
        provider: &(dyn Provider + Send + Sync),
        name: &str,
    ) -> Option<Address> {
        let node = namehash(name);

        // Step 1: Get the resolver address from the ENS registry
        let resolver_calldata = resolverCall { node }.abi_encode();
        let resolver_tx = TransactionRequest::default()
            .to(ENS_REGISTRY)
            .input(Bytes::from(resolver_calldata).into());

        let resolver_result = provider.call(resolver_tx).await.ok()?;
        let resolver_addr = parse_address_from_result(&resolver_result)?;

        // Zero address means no resolver set
        if resolver_addr == Address::ZERO {
            return None;
        }

        // Step 2: Call the resolver's addr(bytes32) to get the address
        let addr_calldata = addrCall { node }.abi_encode();
        let addr_tx = TransactionRequest::default()
            .to(resolver_addr)
            .input(Bytes::from(addr_calldata).into());

        let addr_result = provider.call(addr_tx).await.ok()?;
        let resolved_addr = parse_address_from_result(&addr_result)?;

        if resolved_addr == Address::ZERO {
            return None;
        }

        Some(resolved_addr)
    }
}

impl Default for EnsResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute the EIP-137 namehash for a given ENS name.
///
/// namehash('') = 0x0000000000000000000000000000000000000000000000000000000000000000
/// namehash('eth') = keccak256(namehash('') + keccak256('eth'))
/// namehash('vitalik.eth') = keccak256(namehash('eth') + keccak256('vitalik'))
pub fn namehash(name: &str) -> FixedBytes<32> {
    let mut node = B256::ZERO;

    if name.is_empty() {
        return node;
    }

    // Split name into labels and process from right to left
    for label in name.rsplit('.') {
        let label_hash = alloy::primitives::keccak256(label.as_bytes());
        // Concatenate current node + label_hash and hash the result
        let mut combined = [0u8; 64];
        combined[..32].copy_from_slice(node.as_slice());
        combined[32..].copy_from_slice(label_hash.as_slice());
        node = alloy::primitives::keccak256(&combined);
    }

    node
}

/// Parse an ABI-encoded address from a 32-byte call result.
/// The address sits in the last 20 bytes of the 32-byte word.
fn parse_address_from_result(data: &Bytes) -> Option<Address> {
    if data.len() < 32 {
        return None;
    }
    Some(Address::from_slice(&data[12..32]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namehash_empty() {
        let hash = namehash("");
        assert_eq!(hash, B256::ZERO);
    }

    #[test]
    fn test_namehash_eth() {
        let hash = namehash("eth");
        // Known value: namehash('eth') = 0x93cdeb708b7545dc668eb9280176169d1c33cfd8ed6f04690a0bcc88a93fc4ae
        let expected: B256 = "0x93cdeb708b7545dc668eb9280176169d1c33cfd8ed6f04690a0bcc88a93fc4ae"
            .parse()
            .unwrap();
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_namehash_vitalik_eth() {
        let hash = namehash("vitalik.eth");
        // Known value for vitalik.eth
        let expected: B256 = "0xee6c4522aab0003e8d14cd40a6af439055fd2577951148c14b6cea9a53475835"
            .parse()
            .unwrap();
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_parse_address_from_result_valid() {
        let mut data = vec![0u8; 32];
        // Put an address in bytes 12..32
        data[12..32].copy_from_slice(&[0xd8, 0xdA, 0x6B, 0xF2, 0x69, 0x64, 0xaF, 0x9D,
                                       0x7e, 0xEd, 0x9e, 0x03, 0xE5, 0x34, 0x15, 0xD3,
                                       0x7a, 0xA9, 0x60, 0x45]);
        let result = parse_address_from_result(&Bytes::from(data));
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_address_from_result_too_short() {
        let data = Bytes::from(vec![0u8; 10]);
        assert!(parse_address_from_result(&data).is_none());
    }
}
