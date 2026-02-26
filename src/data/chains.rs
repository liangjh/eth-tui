use crate::data::types::ChainConfig;

/// Get a chain configuration preset by name.
pub fn get_chain_config(name: &str) -> Option<ChainConfig> {
    match name.to_lowercase().as_str() {
        "ethereum" | "eth" | "mainnet" => Some(ChainConfig {
            name: "Ethereum".to_string(),
            chain_id: 1,
            rpc_url: "https://eth.merkle.io".to_string(),
            symbol: "ETH".to_string(),
            explorer_url: Some("https://etherscan.io".to_string()),
            explorer_api_key: None,
        }),
        "arbitrum" | "arb" => Some(ChainConfig {
            name: "Arbitrum One".to_string(),
            chain_id: 42161,
            rpc_url: "https://arb1.arbitrum.io/rpc".to_string(),
            symbol: "ETH".to_string(),
            explorer_url: Some("https://arbiscan.io".to_string()),
            explorer_api_key: None,
        }),
        "optimism" | "op" => Some(ChainConfig {
            name: "Optimism".to_string(),
            chain_id: 10,
            rpc_url: "https://mainnet.optimism.io".to_string(),
            symbol: "ETH".to_string(),
            explorer_url: Some("https://optimistic.etherscan.io".to_string()),
            explorer_api_key: None,
        }),
        "base" => Some(ChainConfig {
            name: "Base".to_string(),
            chain_id: 8453,
            rpc_url: "https://mainnet.base.org".to_string(),
            symbol: "ETH".to_string(),
            explorer_url: Some("https://basescan.org".to_string()),
            explorer_api_key: None,
        }),
        "polygon" | "matic" => Some(ChainConfig {
            name: "Polygon".to_string(),
            chain_id: 137,
            rpc_url: "https://polygon-rpc.com".to_string(),
            symbol: "MATIC".to_string(),
            explorer_url: Some("https://polygonscan.com".to_string()),
            explorer_api_key: None,
        }),
        _ => None,
    }
}

/// Return a list of all supported chain names.
pub fn supported_chains() -> Vec<&'static str> {
    vec!["ethereum", "arbitrum", "optimism", "base", "polygon"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ethereum_config() {
        let config = get_chain_config("ethereum").unwrap();
        assert_eq!(config.chain_id, 1);
        assert_eq!(config.symbol, "ETH");
    }

    #[test]
    fn test_ethereum_aliases() {
        assert!(get_chain_config("eth").is_some());
        assert!(get_chain_config("mainnet").is_some());
        assert!(get_chain_config("Ethereum").is_some());
    }

    #[test]
    fn test_arbitrum_config() {
        let config = get_chain_config("arbitrum").unwrap();
        assert_eq!(config.chain_id, 42161);
        assert_eq!(config.rpc_url, "https://arb1.arbitrum.io/rpc");
    }

    #[test]
    fn test_optimism_config() {
        let config = get_chain_config("optimism").unwrap();
        assert_eq!(config.chain_id, 10);
        assert_eq!(config.rpc_url, "https://mainnet.optimism.io");
    }

    #[test]
    fn test_base_config() {
        let config = get_chain_config("base").unwrap();
        assert_eq!(config.chain_id, 8453);
        assert_eq!(config.rpc_url, "https://mainnet.base.org");
    }

    #[test]
    fn test_polygon_config() {
        let config = get_chain_config("polygon").unwrap();
        assert_eq!(config.chain_id, 137);
        assert_eq!(config.symbol, "MATIC");
    }

    #[test]
    fn test_polygon_alias() {
        assert!(get_chain_config("matic").is_some());
    }

    #[test]
    fn test_unknown_chain() {
        assert!(get_chain_config("unknown").is_none());
    }

    #[test]
    fn test_supported_chains() {
        let chains = supported_chains();
        assert_eq!(chains.len(), 5);
        assert!(chains.contains(&"ethereum"));
        assert!(chains.contains(&"polygon"));
    }
}
