use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::components::Component;
use crate::data::types::TransactionSummary;
use crate::events::{AppEvent, View};
use crate::theme::THEME;
use crate::utils;

pub struct MempoolView {
    pub pending_txs: Vec<TransactionSummary>,
    pub selected: usize,
    pub connected: bool,
    pub loading: bool,
    table_state: TableState,
    scroll_state: ScrollbarState,
}

impl MempoolView {
    pub fn new() -> Self {
        Self {
            pending_txs: Vec::new(),
            selected: 0,
            connected: false,
            loading: false,
            table_state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::default(),
        }
    }

    /// Add a pending transaction. Keeps list sorted by gas price descending.
    pub fn add_pending_tx(&mut self, tx: TransactionSummary) {
        self.pending_txs.push(tx);
        self.sort_by_gas_price();
    }

    /// Replace the full pending tx list.
    pub fn set_pending_txs(&mut self, txs: Vec<TransactionSummary>) {
        self.pending_txs = txs;
        self.sort_by_gas_price();
        if self.selected >= self.pending_txs.len() && !self.pending_txs.is_empty() {
            self.selected = self.pending_txs.len() - 1;
        }
        self.table_state.select(Some(self.selected));
    }

    fn sort_by_gas_price(&mut self) {
        self.pending_txs
            .sort_by(|a, b| b.gas_price.unwrap_or(0).cmp(&a.gas_price.unwrap_or(0)));
    }

    fn select_next(&mut self) {
        if self.pending_txs.is_empty() {
            return;
        }
        let next = if self.selected + 1 >= self.pending_txs.len() {
            self.selected
        } else {
            self.selected + 1
        };
        self.selected = next;
        self.table_state.select(Some(next));
        self.scroll_state = self.scroll_state.position(next);
    }

    fn select_prev(&mut self) {
        if self.pending_txs.is_empty() {
            return;
        }
        let prev = self.selected.saturating_sub(1);
        self.selected = prev;
        self.table_state.select(Some(prev));
        self.scroll_state = self.scroll_state.position(prev);
    }

    fn select_first(&mut self) {
        if self.pending_txs.is_empty() {
            return;
        }
        self.selected = 0;
        self.table_state.select(Some(0));
        self.scroll_state = self.scroll_state.position(0);
    }

    fn select_last(&mut self) {
        if self.pending_txs.is_empty() {
            return;
        }
        let last = self.pending_txs.len() - 1;
        self.selected = last;
        self.table_state.select(Some(last));
        self.scroll_state = self.scroll_state.position(last);
    }

    fn page_down(&mut self) {
        if self.pending_txs.is_empty() {
            return;
        }
        let last = self.pending_txs.len() - 1;
        self.selected = (self.selected + 20).min(last);
        self.table_state.select(Some(self.selected));
        self.scroll_state = self.scroll_state.position(self.selected);
    }

    fn page_up(&mut self) {
        if self.pending_txs.is_empty() {
            return;
        }
        self.selected = self.selected.saturating_sub(20);
        self.table_state.select(Some(self.selected));
        self.scroll_state = self.scroll_state.position(self.selected);
    }
}

impl Component for MempoolView {
    fn handle_key(&mut self, key: KeyEvent) -> Option<AppEvent> {
        match (key.code, key.modifiers) {
            (KeyCode::Char('j'), _) | (KeyCode::Down, _) => {
                self.select_next();
                None
            }
            (KeyCode::Char('k'), _) | (KeyCode::Up, _) => {
                self.select_prev();
                None
            }
            (KeyCode::Char('g'), _) => {
                self.select_first();
                None
            }
            (KeyCode::Char('G'), _) => {
                self.select_last();
                None
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                self.page_down();
                None
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.page_up();
                None
            }
            (KeyCode::Char('r'), _) => {
                // Refresh — the app layer will re-fetch pending txs
                None
            }
            (KeyCode::Enter, _) => {
                if let Some(tx) = self.pending_txs.get(self.selected) {
                    return Some(AppEvent::Navigate(View::TransactionDetail(tx.hash)));
                }
                None
            }
            (KeyCode::Esc, _) | (KeyCode::Backspace, _) => Some(AppEvent::Back),
            _ => None,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let status = if self.connected { "Connected" } else { "Disconnected" };
        let title = format!(" Mempool ({}) [{}] ", self.pending_txs.len(), status);

        let outer_block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(THEME.border_focused_style());

        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        if !self.connected {
            let text = Paragraph::new(
                "WebSocket not connected.\n\nUse --ws-url to enable mempool viewing.",
            )
            .style(THEME.muted_style())
            .alignment(Alignment::Center);
            frame.render_widget(text, inner);
            return;
        }

        if self.pending_txs.is_empty() {
            let msg = if self.loading {
                "Fetching pending transactions..."
            } else {
                "Waiting for pending transactions..."
            };
            let text = Paragraph::new(msg)
                .style(THEME.muted_style())
                .alignment(Alignment::Center);
            frame.render_widget(text, inner);
            return;
        }

        // -- Transaction table --
        let header = Row::new(vec![
            Cell::from("#"),
            Cell::from("Hash"),
            Cell::from("From"),
            Cell::from("To"),
            Cell::from("Value"),
            Cell::from("Gas Price"),
            Cell::from("Method"),
        ])
        .style(THEME.table_header_style())
        .bottom_margin(0);

        let rows: Vec<Row> = self
            .pending_txs
            .iter()
            .enumerate()
            .map(|(i, tx)| {
                let to_str = tx
                    .to
                    .as_ref()
                    .map(|a| utils::truncate_address(a))
                    .unwrap_or_else(|| "Create".to_string());

                let gas_price_str = tx
                    .gas_price
                    .map(|gp| utils::format_gwei(gp))
                    .unwrap_or_else(|| "N/A".to_string());

                let method = tx
                    .method_name
                    .clone()
                    .or_else(|| tx.method_id.as_ref().map(|id| utils::format_selector(id)))
                    .unwrap_or_else(|| "Transfer".to_string());

                Row::new(vec![
                    Cell::from(format!("{}", i + 1)),
                    Cell::from(utils::truncate_hash(&tx.hash)).style(THEME.hash_style()),
                    Cell::from(utils::truncate_address(&tx.from)).style(THEME.address_style()),
                    Cell::from(to_str).style(THEME.address_style()),
                    Cell::from(utils::format_eth(tx.value)).style(THEME.eth_style()),
                    Cell::from(gas_price_str).style(Style::default().fg(THEME.warning)),
                    Cell::from(method).style(THEME.muted_style()),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(5),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(16),
            Constraint::Length(14),
            Constraint::Min(10),
        ];

        self.scroll_state = self.scroll_state.content_length(self.pending_txs.len());

        let table = Table::new(rows, widths)
            .header(header)
            .row_highlight_style(THEME.selected_style())
            .highlight_symbol(" > ");

        frame.render_stateful_widget(table, inner, &mut self.table_state);

        // Scrollbar
        if self.pending_txs.len() > inner.height as usize {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("^"))
                .end_symbol(Some("v"));

            let scrollbar_area = Rect {
                x: area.x + area.width.saturating_sub(1),
                y: area.y + 1,
                width: 1,
                height: area.height.saturating_sub(2),
            };

            frame.render_stateful_widget(scrollbar, scrollbar_area, &mut self.scroll_state);
        }
    }
}
