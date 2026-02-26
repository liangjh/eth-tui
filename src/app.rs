use std::sync::Arc;
use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use futures::StreamExt;
use ratatui::prelude::*;
use ratatui::widgets::*;
use tokio::sync::mpsc;

use crate::components::address_view::AddressView;
use crate::components::block_detail::BlockDetailView;
use crate::components::block_list::BlockList;
use crate::components::dashboard::Dashboard;
use crate::components::gas_tracker::GasTracker;
use crate::components::header::Header;
use crate::components::help::HelpOverlay;
use crate::components::search::SearchBar;
use crate::components::status_bar::StatusBar;
use crate::components::Component;
use crate::data::DataService;
use crate::events::{AppEvent, View};
use crate::theme::THEME;

pub struct App {
    // Navigation
    view_stack: Vec<View>,
    current_view: View,

    // Components
    header: Header,
    dashboard: Dashboard,
    block_list: BlockList,
    block_detail: BlockDetailView,
    tx_detail: crate::components::tx_detail::TxDetailView,
    address_view: AddressView,
    gas_tracker: GasTracker,
    status_bar: StatusBar,
    search_bar: SearchBar,
    help: HelpOverlay,

    // Data
    data_service: Arc<DataService>,
    event_rx: mpsc::UnboundedReceiver<AppEvent>,

    // State
    should_quit: bool,
    tick_rate: Duration,
}

impl App {
    pub fn with_service(
        data_service: Arc<DataService>,
        event_rx: mpsc::UnboundedReceiver<AppEvent>,
        tick_rate_ms: u64,
    ) -> Self {
        Self {
            view_stack: Vec::new(),
            current_view: View::Dashboard,
            header: Header::new(),
            dashboard: Dashboard::new(),
            block_list: BlockList::new(),
            block_detail: BlockDetailView::new(),
            tx_detail: crate::components::tx_detail::TxDetailView::new(),
            address_view: AddressView::new(),
            gas_tracker: GasTracker::new(),
            status_bar: StatusBar::new(),
            search_bar: SearchBar::new(),
            help: HelpOverlay::new(),
            data_service,
            event_rx,
            should_quit: false,
            tick_rate: Duration::from_millis(tick_rate_ms),
        }
    }

    pub async fn run(&mut self, mut terminal: ratatui::DefaultTerminal) -> color_eyre::Result<()> {
        // Initial data load
        self.data_service.fetch_latest_block_number();
        self.data_service.fetch_recent_blocks(20);
        self.data_service.fetch_gas_info();

        let mut interval = tokio::time::interval(self.tick_rate);
        let mut events = EventStream::new();

        while !self.should_quit {
            tokio::select! {
                _ = interval.tick() => {
                    terminal.draw(|frame| self.render(frame))?;
                }
                Some(Ok(event)) = events.next() => {
                    self.handle_terminal_event(event);
                }
                Some(app_event) = self.event_rx.recv() => {
                    self.handle_app_event(app_event);
                }
            }
        }

        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Fill background
        frame.render_widget(
            Block::default().style(Style::default().bg(THEME.bg)),
            area,
        );

        // Layout: header (1) | content (fill) | status bar (1)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

        // Header
        self.header.render(frame, chunks[0]);

        // Main content based on current view
        match &self.current_view {
            View::Dashboard => self.dashboard.render(frame, chunks[1]),
            View::BlockList => self.block_list.render(frame, chunks[1]),
            View::BlockDetail(_) => self.block_detail.render(frame, chunks[1]),
            View::TransactionDetail(_) => self.tx_detail.render(frame, chunks[1]),
            View::AddressView(_) => self.address_view.render(frame, chunks[1]),
            View::GasTracker => self.gas_tracker.render(frame, chunks[1]),
        }

        // Status bar
        self.status_bar.render(frame, chunks[2]);

        // Overlays (rendered on top)
        self.search_bar.render(frame, area);
        self.help.render(frame, area);
    }

    fn handle_terminal_event(&mut self, event: Event) {
        if let Event::Key(key) = event {
            // Only handle key press events (not release/repeat) for cross-platform compat
            if key.kind != KeyEventKind::Press {
                return;
            }

            // Help overlay consumes all keys when visible
            if self.help.handle_key(key) {
                return;
            }

            // Search bar consumes keys when active
            if self.search_bar.active {
                if let Some(query) = self.search_bar.handle_key(key) {
                    if !query.is_empty() {
                        self.status_bar.loading = true;
                        self.data_service.search(query);
                    }
                }
                return;
            }

            // Global keys
            match key.code {
                KeyCode::Char('q') => {
                    self.should_quit = true;
                    return;
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.should_quit = true;
                    return;
                }
                KeyCode::Char('/') | KeyCode::Char('s') => {
                    self.search_bar.activate();
                    return;
                }
                KeyCode::Char('?') => {
                    self.help.toggle();
                    return;
                }
                // Tab switching with number keys
                KeyCode::Char('1') => {
                    self.navigate_to(View::Dashboard);
                    return;
                }
                KeyCode::Char('2') => {
                    self.navigate_to(View::BlockList);
                    return;
                }
                KeyCode::Char('3') => {
                    self.navigate_to(View::GasTracker);
                    return;
                }
                KeyCode::Esc | KeyCode::Backspace => {
                    self.go_back();
                    return;
                }
                _ => {}
            }

            // Delegate to current view's component
            let app_event = match &self.current_view {
                View::Dashboard => self.dashboard.handle_key(key),
                View::BlockList => self.block_list.handle_key(key),
                View::BlockDetail(_) => self.block_detail.handle_key(key),
                View::TransactionDetail(_) => self.tx_detail.handle_key(key),
                View::AddressView(_) => self.address_view.handle_key(key),
                View::GasTracker => self.gas_tracker.handle_key(key),
            };

            if let Some(event) = app_event {
                self.handle_app_event(event);
            }
        }
    }

    fn handle_app_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Connected(chain_id) => {
                self.header.chain_id = chain_id;
                self.header.connected = true;
                self.status_bar.connected = true;
            }
            AppEvent::LatestBlockNumber(number) => {
                self.header.latest_block = number;
                self.status_bar.latest_block = number;
                self.header.connected = true;
                self.status_bar.connected = true;
            }
            AppEvent::RecentBlocks(blocks) => {
                self.status_bar.loading = false;
                // Also extract transactions from blocks for dashboard
                let txs: Vec<_> = blocks
                    .iter()
                    .take(5)
                    .flat_map(|_| Vec::<crate::data::types::TransactionSummary>::new())
                    .collect();
                self.dashboard.blocks = blocks.clone();
                self.block_list.blocks = blocks;
                if txs.is_empty() {
                    // Transactions come separately or from block details
                }
            }
            AppEvent::RecentTransactions(txs) => {
                self.dashboard.transactions = txs;
            }
            AppEvent::BlockDetailLoaded(detail) => {
                self.status_bar.loading = false;
                // Also populate dashboard transactions from block detail txs
                if self.dashboard.transactions.is_empty() && !detail.transactions.is_empty() {
                    self.dashboard.transactions = detail.transactions.clone();
                }
                self.block_detail.detail = Some(*detail);
                self.block_detail.loading = false;
            }
            AppEvent::TransactionDetailLoaded(detail) => {
                self.status_bar.loading = false;
                self.tx_detail.detail = Some(*detail);
                self.tx_detail.loading = false;
            }
            AppEvent::AddressInfoLoaded(info) => {
                self.status_bar.loading = false;
                self.address_view.info = Some(*info);
                self.address_view.loading = false;
            }
            AppEvent::GasInfoLoaded(info) => {
                self.gas_tracker.info = Some(info);
                self.gas_tracker.loading = false;
            }
            AppEvent::SearchResult(_target) => {
                self.status_bar.loading = false;
                self.search_bar.deactivate();
            }
            AppEvent::SearchNotFound(msg) => {
                self.status_bar.loading = false;
                self.search_bar.error = Some(msg.clone());
                self.search_bar.active = true;
                self.status_bar.error_message = Some(msg);
            }
            AppEvent::Navigate(view) => {
                self.navigate_to(view);
            }
            AppEvent::Back => {
                self.go_back();
            }
            AppEvent::Error(msg) => {
                self.status_bar.error_message = Some(msg);
                self.status_bar.loading = false;
            }
        }
    }

    fn navigate_to(&mut self, view: View) {
        // Update tab indicator
        match &view {
            View::Dashboard => self.header.current_tab = 0,
            View::BlockList | View::BlockDetail(_) => self.header.current_tab = 1,
            View::GasTracker => self.header.current_tab = 2,
            _ => {} // Keep current tab for tx/address detail views
        }

        // Clear error on navigation
        self.status_bar.error_message = None;

        // Push current view to stack
        let old_view = std::mem::replace(&mut self.current_view, view.clone());
        self.view_stack.push(old_view);

        // Trigger data loading for the new view
        match &view {
            View::Dashboard => {
                self.data_service.fetch_recent_blocks(20);
            }
            View::BlockList => {
                if self.block_list.blocks.is_empty() {
                    self.status_bar.loading = true;
                    self.data_service.fetch_recent_blocks(50);
                }
            }
            View::BlockDetail(number) => {
                self.block_detail.detail = None;
                self.block_detail.loading = true;
                self.status_bar.loading = true;
                self.data_service.fetch_block_detail(*number);
            }
            View::TransactionDetail(hash) => {
                self.tx_detail.detail = None;
                self.tx_detail.loading = true;
                self.status_bar.loading = true;
                self.data_service.fetch_transaction_detail(*hash);
            }
            View::AddressView(address) => {
                self.address_view.info = None;
                self.address_view.loading = true;
                self.status_bar.loading = true;
                self.data_service.fetch_address_info(*address);
            }
            View::GasTracker => {
                if self.gas_tracker.info.is_none() {
                    self.gas_tracker.loading = true;
                    self.data_service.fetch_gas_info();
                }
            }
        }
    }

    fn go_back(&mut self) {
        if let Some(prev_view) = self.view_stack.pop() {
            self.current_view = prev_view;
            match &self.current_view {
                View::Dashboard => self.header.current_tab = 0,
                View::BlockList | View::BlockDetail(_) => self.header.current_tab = 1,
                View::GasTracker => self.header.current_tab = 2,
                _ => {}
            }
            self.status_bar.error_message = None;
        }
    }
}
