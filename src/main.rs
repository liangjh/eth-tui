mod app;
mod components;
mod config;
mod data;
mod events;
mod theme;
mod utils;

use std::sync::Arc;

use clap::Parser;
use color_eyre::eyre::Result;
use tokio::sync::mpsc;

use crate::app::App;
use crate::config::Config;
use crate::data::provider::EthProvider;
use crate::data::DataService;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let config = Config::parse();

    // Resolve RPC URL: use chain preset if not default ethereum
    let rpc_url = if config.chain != "ethereum" {
        if let Some(chain_config) = data::chains::get_chain_config(&config.chain) {
            chain_config.rpc_url
        } else {
            eprintln!("Unknown chain '{}', using default RPC", config.chain);
            config.rpc_url.clone()
        }
    } else {
        config.rpc_url.clone()
    };

    // Connect to the Ethereum node
    eprintln!("Connecting to {}...", rpc_url);
    let provider = EthProvider::connect(&rpc_url).await?;
    let chain_id = provider.chain_id();
    eprintln!("Connected to chain {} (block data loading...)", chain_id);

    // Create event channel
    let (event_tx, event_rx) = mpsc::unbounded_channel();

    // Send initial connected event
    let _ = event_tx.send(events::AppEvent::Connected(chain_id));

    // Create data service
    let data_service = Arc::new(DataService::new(
        provider,
        config.etherscan_api_key,
        event_tx.clone(),
    ));

    // Create app
    let mut app = App::with_service(data_service, event_rx, config.tick_rate_ms);

    // Set chain info on header
    if let Some(chain_config) = data::chains::get_chain_config(&config.chain) {
        app.set_chain_info(chain_config.name, chain_config.symbol);
    }

    // Create WsService if ws_url is provided
    if let Some(ref ws_url) = config.ws_url {
        let _ws_service = data::ws::WsService::new(event_tx.clone());
        eprintln!("WebSocket URL configured: {ws_url}");
    }

    // Handle initial search if provided - queue it for after event loop starts
    if let Some(ref query) = config.search {
        if let Some(target) = events::SearchTarget::parse(query) {
            let view = match target {
                events::SearchTarget::BlockNumber(n) => events::View::BlockDetail(n),
                events::SearchTarget::TransactionHash(h) => events::View::TransactionDetail(h),
                events::SearchTarget::Address(a) => events::View::AddressView(a),
                events::SearchTarget::BlockHash(_h) => events::View::BlockDetail(0_u64), // Will be resolved by search
                events::SearchTarget::EnsName(_) => {
                    // ENS resolution needs the event loop running - use search
                    let _ = event_tx.send(events::AppEvent::Error(
                        "ENS resolution requires event loop".to_string(),
                    ));
                    events::View::Dashboard
                }
            };
            // For block hash, use search instead of direct navigation
            if matches!(
                events::SearchTarget::parse(query),
                Some(events::SearchTarget::BlockHash(_))
            ) {
                // Search will handle block hash resolution
                let event_tx_clone = event_tx.clone();
                tokio::spawn(async move {
                    // Small delay to ensure the event loop is running
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    let _ = event_tx_clone.send(events::AppEvent::Navigate(events::View::Dashboard));
                });
            } else {
                let _ = event_tx.send(events::AppEvent::Navigate(view));
            }
        } else {
            eprintln!("Could not parse search query: {query}");
        }
    }

    // Initialize terminal
    let terminal = ratatui::init();
    let result = app.run(terminal).await;

    // Restore terminal
    ratatui::restore();

    result
}
