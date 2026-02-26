use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "eth-tui", about = "Terminal Ethereum blockchain explorer")]
pub struct Config {
    /// RPC endpoint URL (http://, ws://, ipc://)
    #[arg(short, long, default_value = "https://eth.merkle.io")]
    pub rpc_url: String,

    /// Etherscan API key (optional, for ABI resolution)
    #[arg(long, env = "ETHERSCAN_API_KEY")]
    pub etherscan_api_key: Option<String>,

    /// Start with a specific search query
    #[arg(short, long)]
    pub search: Option<String>,

    /// Tick rate in milliseconds for UI refresh
    #[arg(long, default_value = "100")]
    pub tick_rate_ms: u64,

    /// WebSocket RPC endpoint URL for live subscriptions
    #[arg(long)]
    pub ws_url: Option<String>,

    /// Chain preset (ethereum, arbitrum, optimism, base, polygon)
    #[arg(long, default_value = "ethereum")]
    pub chain: String,
}
