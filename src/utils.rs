use alloy::primitives::{Address, B256, U256};
use chrono::{DateTime, Utc};

/// Truncate a B256 hash to "0xabcd...ef12" format
pub fn truncate_hash(hash: &B256) -> String {
    let s = format!("{hash}");
    if s.len() > 14 {
        format!("{}...{}", &s[..8], &s[s.len() - 4..])
    } else {
        s
    }
}

/// Truncate an address to "0xabcd...ef12" format
pub fn truncate_address(addr: &Address) -> String {
    let s = format!("{addr}");
    if s.len() > 14 {
        format!("{}...{}", &s[..8], &s[s.len() - 4..])
    } else {
        s
    }
}

/// Format a U256 wei value as ETH with reasonable precision
pub fn format_eth(wei: U256) -> String {
    let eth_str = format_u256_as_decimal(wei, 18);
    format!("{eth_str} ETH")
}

/// Format a U256 value as decimal with given decimals
pub fn format_u256_as_decimal(value: U256, decimals: u8) -> String {
    if value.is_zero() {
        return "0.0".to_string();
    }

    let divisor = U256::from(10u64).pow(U256::from(decimals));
    let whole = value / divisor;
    let remainder = value % divisor;

    if remainder.is_zero() {
        return format!("{whole}.0");
    }

    let remainder_str = format!("{remainder}");
    let padded = format!("{:0>width$}", remainder_str, width = decimals as usize);
    let trimmed = padded.trim_end_matches('0');

    // Limit to 6 decimal places
    let decimals_shown = trimmed.len().min(6);
    format!("{whole}.{}", &trimmed[..decimals_shown])
}

/// Format gas in Gwei
pub fn format_gwei(wei: u128) -> String {
    let gwei = wei as f64 / 1e9;
    if gwei < 0.01 {
        format!("{gwei:.4} Gwei")
    } else if gwei < 10.0 {
        format!("{gwei:.2} Gwei")
    } else {
        format!("{gwei:.1} Gwei")
    }
}

/// Format gas usage as "12,500,000 (63.2%)"
pub fn format_gas_usage(used: u64, limit: u64) -> String {
    let pct = if limit > 0 {
        (used as f64 / limit as f64) * 100.0
    } else {
        0.0
    };
    format!("{} ({pct:.1}%)", format_number(used))
}

/// Format a number with comma separators
pub fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

/// Format a Unix timestamp as "Xm ago", "Xh ago", etc.
pub fn format_time_ago(timestamp: u64) -> String {
    let now = Utc::now().timestamp() as u64;
    if timestamp > now {
        return "just now".to_string();
    }
    let diff = now - timestamp;
    if diff < 60 {
        format!("{diff}s ago")
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else {
        format!("{}d ago", diff / 86400)
    }
}

/// Format a Unix timestamp as a datetime string
pub fn format_timestamp(timestamp: u64) -> String {
    DateTime::from_timestamp(timestamp as i64, 0)
        .map(|dt| dt.format("%b %d, %Y %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

/// Format a method selector as "0xabcdef12"
pub fn format_selector(selector: &[u8; 4]) -> String {
    format!("0x{}", hex::encode(selector))
}

/// Determine gas utilization percentage
pub fn gas_utilization_pct(used: u64, limit: u64) -> f64 {
    if limit == 0 {
        return 0.0;
    }
    (used as f64 / limit as f64) * 100.0
}

mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}
