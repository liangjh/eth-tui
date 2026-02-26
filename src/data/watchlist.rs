use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use alloy::primitives::Address;

use crate::data::types::WatchEntry;

const WATCHLIST_FILE: &str = "watchlist.json";
const APP_DIR: &str = "eth-tui";

/// Persistent watch list stored on disk at ~/.config/eth-tui/watchlist.json.
pub struct WatchList {
    pub entries: Vec<WatchEntry>,
}

impl WatchList {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Load the watchlist from disk. Returns empty list if file doesn't exist.
    pub fn load() -> Self {
        let path = match watchlist_path() {
            Some(p) => p,
            None => return Self::new(),
        };

        let data = match fs::read_to_string(&path) {
            Ok(d) => d,
            Err(_) => return Self::new(),
        };

        let entries: Vec<WatchEntry> = match serde_json::from_str(&data) {
            Ok(e) => e,
            Err(_) => return Self::new(),
        };

        Self { entries }
    }

    /// Save the watchlist to disk.
    pub fn save(&self) -> Result<(), String> {
        let path = watchlist_path().ok_or("Could not determine config directory")?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {e}"))?;
        }

        let json = serde_json::to_string_pretty(&self.entries)
            .map_err(|e| format!("Failed to serialize watchlist: {e}"))?;

        fs::write(&path, json).map_err(|e| format!("Failed to write watchlist: {e}"))?;

        Ok(())
    }

    /// Add an address to the watchlist with a label.
    /// Returns false if the address is already in the watchlist.
    pub fn add(&mut self, address: Address, label: String) -> bool {
        if self.entries.iter().any(|e| e.address == address) {
            return false;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.entries.push(WatchEntry {
            address,
            label,
            added_at: now,
        });

        true
    }

    /// Remove an address from the watchlist.
    /// Returns true if the address was found and removed.
    pub fn remove(&mut self, address: &Address) -> bool {
        let len_before = self.entries.len();
        self.entries.retain(|e| &e.address != address);
        self.entries.len() < len_before
    }

    /// List all watched entries.
    pub fn list(&self) -> &[WatchEntry] {
        &self.entries
    }

    /// Check if an address is in the watchlist.
    pub fn contains(&self, address: &Address) -> bool {
        self.entries.iter().any(|e| &e.address == address)
    }
}

impl Default for WatchList {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the path to the watchlist file.
fn watchlist_path() -> Option<PathBuf> {
    let config_dir = dirs::config_dir()?;
    Some(config_dir.join(APP_DIR).join(WATCHLIST_FILE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_watchlist_empty() {
        let wl = WatchList::new();
        assert!(wl.entries.is_empty());
        assert!(wl.list().is_empty());
    }

    #[test]
    fn test_add_entry() {
        let mut wl = WatchList::new();
        let addr = Address::from_slice(&[0x01; 20]);
        assert!(wl.add(addr, "Test".to_string()));
        assert_eq!(wl.list().len(), 1);
        assert_eq!(wl.list()[0].label, "Test");
    }

    #[test]
    fn test_add_duplicate() {
        let mut wl = WatchList::new();
        let addr = Address::from_slice(&[0x01; 20]);
        assert!(wl.add(addr, "First".to_string()));
        assert!(!wl.add(addr, "Second".to_string()));
        assert_eq!(wl.list().len(), 1);
    }

    #[test]
    fn test_remove_entry() {
        let mut wl = WatchList::new();
        let addr = Address::from_slice(&[0x01; 20]);
        wl.add(addr, "Test".to_string());
        assert!(wl.remove(&addr));
        assert!(wl.list().is_empty());
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut wl = WatchList::new();
        let addr = Address::from_slice(&[0x01; 20]);
        assert!(!wl.remove(&addr));
    }

    #[test]
    fn test_contains() {
        let mut wl = WatchList::new();
        let addr = Address::from_slice(&[0x01; 20]);
        assert!(!wl.contains(&addr));
        wl.add(addr, "Test".to_string());
        assert!(wl.contains(&addr));
    }

    #[test]
    fn test_watchlist_path() {
        let path = watchlist_path();
        // Should return Some on most systems
        if let Some(p) = path {
            assert!(p.to_string_lossy().contains("eth-tui"));
            assert!(p.to_string_lossy().contains("watchlist.json"));
        }
    }
}
