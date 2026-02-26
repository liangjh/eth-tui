use std::num::NonZeroUsize;
use std::sync::{Mutex, OnceLock};

use alloy::json_abi::JsonAbi;
use alloy::primitives::Address;
use lru::LruCache;

/// A resolved ABI along with the source it was obtained from.
#[derive(Debug, Clone)]
pub struct ResolvedAbi {
    pub abi: JsonAbi,
    pub source: String,
}

/// Cascading ABI resolver: Sourcify -> Etherscan -> built-in ERC ABIs.
/// Also resolves 4-byte function selectors via the 4byte.directory API.
pub struct AbiResolver {
    client: reqwest::Client,
    etherscan_api_key: Option<String>,
    cache: Mutex<LruCache<Address, Option<ResolvedAbi>>>,
    selector_cache: Mutex<LruCache<[u8; 4], Option<String>>>,
}

// --- Built-in ABI singletons ---

static ERC20_ABI: OnceLock<JsonAbi> = OnceLock::new();
static ERC721_ABI: OnceLock<JsonAbi> = OnceLock::new();
static ERC1155_ABI: OnceLock<JsonAbi> = OnceLock::new();

fn get_erc20_abi() -> &'static JsonAbi {
    ERC20_ABI.get_or_init(|| {
        serde_json::from_str(include_str!("../../abis/erc20.json"))
            .expect("built-in ERC-20 ABI should be valid")
    })
}

fn get_erc721_abi() -> &'static JsonAbi {
    ERC721_ABI.get_or_init(|| {
        serde_json::from_str(include_str!("../../abis/erc721.json"))
            .expect("built-in ERC-721 ABI should be valid")
    })
}

fn get_erc1155_abi() -> &'static JsonAbi {
    ERC1155_ABI.get_or_init(|| {
        serde_json::from_str(include_str!("../../abis/erc1155.json"))
            .expect("built-in ERC-1155 ABI should be valid")
    })
}

impl AbiResolver {
    pub fn new(etherscan_api_key: Option<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            etherscan_api_key,
            cache: Mutex::new(LruCache::new(NonZeroUsize::new(500).unwrap())),
            selector_cache: Mutex::new(LruCache::new(NonZeroUsize::new(2000).unwrap())),
        }
    }

    /// Resolve an ABI for a contract address using a cascading strategy:
    /// 1. In-memory cache
    /// 2. Sourcify full-match metadata
    /// 3. Etherscan (if API key is configured)
    /// 4. Built-in ERC-20/721/1155 ABIs (returned as fallback)
    pub async fn resolve(&self, chain_id: u64, address: Address) -> Option<ResolvedAbi> {
        // 1. Check cache
        {
            let mut cache = self.cache.lock().ok()?;
            if let Some(cached) = cache.get(&address) {
                return cached.clone();
            }
        }

        // 2. Try Sourcify
        if let Some(resolved) = self.try_sourcify(chain_id, address).await {
            self.cache_abi(address, Some(resolved.clone()));
            return Some(resolved);
        }

        // 3. Try Etherscan
        if let Some(resolved) = self.try_etherscan(address).await {
            self.cache_abi(address, Some(resolved.clone()));
            return Some(resolved);
        }

        // 4. Fall back to built-in ABIs: try each to see if any function matches
        //    We return the ERC-20 ABI as the most common fallback for contracts.
        //    The caller can attempt decoding and see if it succeeds.
        let fallback = ResolvedAbi {
            abi: get_erc20_abi().clone(),
            source: "built-in ERC-20".to_string(),
        };
        // Don't cache the fallback so we can retry external sources later
        Some(fallback)
    }

    /// Resolve a 4-byte function selector to a human-readable signature.
    pub async fn resolve_selector(&self, selector: [u8; 4]) -> Option<String> {
        // Check cache
        {
            let mut cache = self.selector_cache.lock().ok()?;
            if let Some(cached) = cache.get(&selector) {
                return cached.clone();
            }
        }

        let result = self.try_4byte(selector).await;

        // Cache the result (including None to avoid repeated lookups)
        if let Ok(mut cache) = self.selector_cache.lock() {
            cache.put(selector, result.clone());
        }

        result
    }

    /// Try resolving ABI from Sourcify's repository.
    /// GET https://repo.sourcify.dev/contracts/full_match/{chainId}/{address}/metadata.json
    async fn try_sourcify(&self, chain_id: u64, address: Address) -> Option<ResolvedAbi> {
        let url = format!(
            "https://repo.sourcify.dev/contracts/full_match/{chain_id}/{address}/metadata.json"
        );

        let response = self.client.get(&url).send().await.ok()?;
        if !response.status().is_success() {
            return None;
        }

        let body: serde_json::Value = response.json().await.ok()?;
        let abi_value = body.get("output")?.get("abi")?;
        let abi: JsonAbi = serde_json::from_value(abi_value.clone()).ok()?;

        Some(ResolvedAbi {
            abi,
            source: "Sourcify".to_string(),
        })
    }

    /// Try resolving ABI from Etherscan.
    /// GET https://api.etherscan.io/api?module=contract&action=getabi&address={address}&apikey={key}
    async fn try_etherscan(&self, address: Address) -> Option<ResolvedAbi> {
        let api_key = self.etherscan_api_key.as_ref()?;

        let url = format!(
            "https://api.etherscan.io/api?module=contract&action=getabi&address={address}&apikey={api_key}"
        );

        let response = self.client.get(&url).send().await.ok()?;
        if !response.status().is_success() {
            return None;
        }

        let body: serde_json::Value = response.json().await.ok()?;

        // Etherscan returns status "1" on success, the ABI as a JSON string in "result"
        let status = body.get("status")?.as_str()?;
        if status != "1" {
            return None;
        }

        let abi_str = body.get("result")?.as_str()?;
        let abi: JsonAbi = serde_json::from_str(abi_str).ok()?;

        Some(ResolvedAbi {
            abi,
            source: "Etherscan".to_string(),
        })
    }

    /// Try resolving a 4-byte selector from 4byte.directory.
    /// GET https://www.4byte.directory/api/v1/signatures/?hex_signature=0x{selector_hex}
    async fn try_4byte(&self, selector: [u8; 4]) -> Option<String> {
        let hex = selector
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let url = format!(
            "https://www.4byte.directory/api/v1/signatures/?hex_signature=0x{hex}"
        );

        let response = self.client.get(&url).send().await.ok()?;
        if !response.status().is_success() {
            return None;
        }

        let body: serde_json::Value = response.json().await.ok()?;
        let results = body.get("results")?.as_array()?;

        // Return the first (most popular) text signature
        let first = results.first()?;
        let sig = first.get("text_signature")?.as_str()?;
        Some(sig.to_string())
    }

    /// Try to match a selector against built-in ERC ABIs.
    /// Returns the function name if found.
    pub fn match_builtin_selector(&self, selector: [u8; 4]) -> Option<String> {
        for abi in [get_erc20_abi(), get_erc721_abi(), get_erc1155_abi()] {
            for func in abi.functions() {
                if func.selector() == selector {
                    return Some(func.name.clone());
                }
            }
        }
        None
    }

    fn cache_abi(&self, address: Address, resolved: Option<ResolvedAbi>) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.put(address, resolved);
        }
    }
}
