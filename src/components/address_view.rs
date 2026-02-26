use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::components::Component;
use crate::data::types::AddressInfo;
use crate::events::{AppEvent, View};
use crate::theme::THEME;
use crate::utils;

pub struct AddressView {
    pub info: Option<AddressInfo>,
    pub loading: bool,
    tx_table_state: TableState,
}

impl AddressView {
    pub fn new() -> Self {
        Self {
            info: None,
            loading: false,
            tx_table_state: TableState::default(),
        }
    }

    fn tx_count(&self) -> usize {
        self.info
            .as_ref()
            .map(|i| i.transactions.len())
            .unwrap_or(0)
    }

    fn select_next_tx(&mut self) {
        let len = self.tx_count();
        if len == 0 {
            return;
        }
        let current = self.tx_table_state.selected().unwrap_or(0);
        let next = if current + 1 >= len { current } else { current + 1 };
        self.tx_table_state.select(Some(next));
    }

    fn select_prev_tx(&mut self) {
        let len = self.tx_count();
        if len == 0 {
            return;
        }
        let current = self.tx_table_state.selected().unwrap_or(0);
        let prev = current.saturating_sub(1);
        self.tx_table_state.select(Some(prev));
    }
}

fn render_header(info: &AddressInfo) -> Paragraph<'static> {
    let title = if info.is_contract {
        format!("Contract {}", info.address)
    } else {
        format!("Address {}", info.address)
    };

    Paragraph::new(Line::from(vec![Span::styled(
        title,
        Style::default()
            .fg(THEME.text_accent)
            .add_modifier(Modifier::BOLD),
    )]))
}

fn render_info_rows(info: &AddressInfo) -> Vec<Row<'static>> {
    let mut rows = Vec::new();

    // Balance
    rows.push(Row::new(vec![
        Cell::from("Balance").style(THEME.muted_style()),
        Cell::from(utils::format_eth(info.balance)).style(THEME.eth_style()),
    ]));

    // Nonce
    rows.push(Row::new(vec![
        Cell::from("Nonce").style(THEME.muted_style()),
        Cell::from(format!("{}", info.nonce)),
    ]));

    // Type
    let type_str = if info.is_contract {
        if let Some(ref ci) = info.contract_info {
            if let Some(ref ct) = ci.contract_type {
                format!("Contract ({})", ct)
            } else {
                "Contract".to_string()
            }
        } else {
            "Contract".to_string()
        }
    } else {
        "EOA (Externally Owned Account)".to_string()
    };
    rows.push(Row::new(vec![
        Cell::from("Type").style(THEME.muted_style()),
        Cell::from(type_str),
    ]));

    // Contract-specific info
    if let Some(ref ci) = info.contract_info {
        if let Some(ref source) = ci.abi_source {
            rows.push(Row::new(vec![
                Cell::from("ABI Source").style(THEME.muted_style()),
                Cell::from(source.clone()),
            ]));
        }

        if let Some(ref name) = ci.name {
            rows.push(Row::new(vec![
                Cell::from("Name").style(THEME.muted_style()),
                Cell::from(name.clone()).style(THEME.accent_style()),
            ]));
        }

        if let Some(ref symbol) = ci.symbol {
            rows.push(Row::new(vec![
                Cell::from("Symbol").style(THEME.muted_style()),
                Cell::from(symbol.clone()).style(THEME.accent_style()),
            ]));
        }

        if let Some(decimals) = ci.decimals {
            rows.push(Row::new(vec![
                Cell::from("Decimals").style(THEME.muted_style()),
                Cell::from(format!("{decimals}")),
            ]));
        }

        if ci.is_proxy {
            let impl_str = ci
                .implementation
                .map(|a| format!("{a}"))
                .unwrap_or_else(|| "Unknown".to_string());
            rows.push(Row::new(vec![
                Cell::from("Proxy Target").style(THEME.muted_style()),
                Cell::from(impl_str).style(THEME.address_style()),
            ]));
        }
    }

    rows
}

fn build_tx_rows(info: &AddressInfo) -> Vec<Row<'static>> {
    info.transactions
        .iter()
        .map(|tx| {
            let to_str = tx
                .to
                .as_ref()
                .map(|a| utils::truncate_address(a))
                .unwrap_or_else(|| "Contract".to_string());
            let from_to = format!("{}  {}", utils::truncate_address(&tx.from), to_str);
            let method = tx
                .method_name
                .clone()
                .or_else(|| tx.method_id.as_ref().map(|id| utils::format_selector(id)))
                .unwrap_or_else(|| "Transfer".to_string());

            Row::new(vec![
                Cell::from(utils::truncate_hash(&tx.hash)).style(THEME.hash_style()),
                Cell::from(from_to).style(THEME.address_style()),
                Cell::from(utils::format_eth(tx.value)).style(THEME.eth_style()),
                Cell::from(method).style(THEME.muted_style()),
            ])
        })
        .collect()
}

/// Helper to count the number of info rows for layout calculation.
fn info_row_count(info: &AddressInfo) -> usize {
    let mut count = 3; // Balance, Nonce, Type are always present
    if let Some(ref ci) = info.contract_info {
        if ci.abi_source.is_some() {
            count += 1;
        }
        if ci.name.is_some() {
            count += 1;
        }
        if ci.symbol.is_some() {
            count += 1;
        }
        if ci.decimals.is_some() {
            count += 1;
        }
        if ci.is_proxy {
            count += 1;
        }
    }
    count
}

impl Component for AddressView {
    fn handle_key(&mut self, key: KeyEvent) -> Option<AppEvent> {
        match key.code {
            KeyCode::Esc | KeyCode::Backspace => Some(AppEvent::Back),
            KeyCode::Char('j') | KeyCode::Down => {
                self.select_next_tx();
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.select_prev_tx();
                None
            }
            KeyCode::Enter => {
                if let Some(info) = &self.info {
                    if let Some(idx) = self.tx_table_state.selected() {
                        if let Some(tx) = info.transactions.get(idx) {
                            return Some(AppEvent::Navigate(View::TransactionDetail(tx.hash)));
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let outer_block = Block::default()
            .title(" Address ")
            .borders(Borders::ALL)
            .border_style(THEME.border_focused_style());

        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        // Show loading state
        if self.loading && self.info.is_none() {
            let loading = Paragraph::new("Loading...")
                .style(THEME.muted_style())
                .alignment(Alignment::Center);
            frame.render_widget(loading, inner);
            return;
        }

        let info = match &self.info {
            Some(i) => i.clone(),
            None => return,
        };

        let has_txs = !info.transactions.is_empty();
        let row_count = info_row_count(&info);

        let constraints = if has_txs {
            vec![
                Constraint::Length(2),                    // address header
                Constraint::Length(row_count as u16 + 1), // info section
                Constraint::Min(6),                       // transaction table
            ]
        } else {
            vec![
                Constraint::Length(2),
                Constraint::Min(4),
                Constraint::Length(0),
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        // -- 1. Address header --
        let header = render_header(&info);
        frame.render_widget(header, chunks[0]);

        // -- 2. Info section --
        let info_rows = render_info_rows(&info);
        let info_widths = [Constraint::Length(14), Constraint::Min(20)];
        let info_block = Block::default().borders(Borders::NONE);
        let info_table = Table::new(info_rows, info_widths).block(info_block);
        frame.render_widget(info_table, chunks[1]);

        // -- 3. Transaction table --
        if has_txs {
            let tx_block = Block::default()
                .title(format!(" Transactions ({}) ", info.transactions.len()))
                .borders(Borders::ALL)
                .border_style(THEME.border_style());

            let tx_header = Row::new(vec![
                Cell::from("Hash"),
                Cell::from("From / To"),
                Cell::from("Value"),
                Cell::from("Method"),
            ])
            .style(THEME.table_header_style())
            .bottom_margin(0);

            let tx_rows = build_tx_rows(&info);
            let tx_widths = [
                Constraint::Length(14),
                Constraint::Min(24),
                Constraint::Length(16),
                Constraint::Length(12),
            ];

            let tx_table = Table::new(tx_rows, tx_widths)
                .header(tx_header)
                .block(tx_block)
                .row_highlight_style(THEME.selected_style())
                .highlight_symbol(" > ");

            frame.render_stateful_widget(tx_table, chunks[2], &mut self.tx_table_state);
        }
    }
}
