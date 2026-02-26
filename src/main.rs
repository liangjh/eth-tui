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

    // Connect to the Ethereum node
    eprintln!("Connecting to {}...", config.rpc_url);
    let provider = EthProvider::connect(&config.rpc_url).await?;
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
        event_tx,
    ));

    // Create app
    let mut app = App::with_service(data_service, event_rx, config.tick_rate_ms);

    // Handle initial search if provided
    if let Some(ref query) = config.search {
        // Will be processed after the event loop starts
        let _query = query.clone();
        // TODO: queue initial search
    }

    // Initialize terminal
    let terminal = ratatui::init();
    let result = app.run(terminal).await;

    // Restore terminal
    ratatui::restore();

    result
}
