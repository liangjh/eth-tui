pub mod address_view;
pub mod block_detail;
pub mod block_list;
pub mod contract_read;
pub mod dashboard;
pub mod gas_tracker;
pub mod header;
pub mod help;
pub mod mempool;
pub mod search;
pub mod status_bar;
pub mod storage_inspector;
pub mod tx_debugger;
pub mod tx_detail;
pub mod watch_list;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::events::AppEvent;

/// Trait for all UI components
pub trait Component {
    /// Handle a key event, optionally returning an AppEvent
    fn handle_key(&mut self, key: KeyEvent) -> Option<AppEvent>;

    /// Render the component into the given area
    fn render(&mut self, frame: &mut Frame, area: Rect);
}
