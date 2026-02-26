use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::components::Component;
use crate::data::types::BlockDetail;
use crate::events::{AppEvent, View};
use crate::theme::THEME;
use crate::utils;

pub struct BlockDetailView {
    pub detail: Option<BlockDetail>,
    pub loading: bool,
    tx_table_state: TableState,
    scroll_offset: u16,
}

impl BlockDetailView {
    pub fn new() -> Self {
        Self {
            detail: None,
            loading: false,
            tx_table_state: TableState::default(),
            scroll_offset: 0,
        }
    }

    fn tx_count(&self) -> usize {
        self.detail
            .as_ref()
            .map(|d| d.transactions.len())
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

fn render_info_section(detail: &BlockDetail) -> Vec<Row<'static>> {
    let s = &detail.summary;
    let gas_pct = utils::gas_utilization_pct(s.gas_used, s.gas_limit);
    let base_fee_str = s
        .base_fee
        .map(|fee| utils::format_gwei(fee))
        .unwrap_or_else(|| "N/A".to_string());

    let mut rows = vec![
        Row::new(vec![
            Cell::from("Block Height").style(THEME.muted_style()),
            Cell::from(format!("{}", s.number)).style(THEME.accent_style()),
            Cell::from("Hash").style(THEME.muted_style()),
            Cell::from(format!("{}", s.hash)).style(THEME.hash_style()),
        ]),
        Row::new(vec![
            Cell::from("Parent Hash").style(THEME.muted_style()),
            Cell::from(format!("{}", detail.parent_hash)).style(THEME.hash_style()),
            Cell::from("Timestamp").style(THEME.muted_style()),
            Cell::from(utils::format_timestamp(s.timestamp)),
        ]),
        Row::new(vec![
            Cell::from("Time Ago").style(THEME.muted_style()),
            Cell::from(utils::format_time_ago(s.timestamp)),
            Cell::from("Transactions").style(THEME.muted_style()),
            Cell::from(format!("{}", s.tx_count)),
        ]),
        Row::new(vec![
            Cell::from("Gas Used").style(THEME.muted_style()),
            Cell::from(utils::format_gas_usage(s.gas_used, s.gas_limit))
                .style(THEME.gas_style(gas_pct)),
            Cell::from("Gas Limit").style(THEME.muted_style()),
            Cell::from(utils::format_number(s.gas_limit)),
        ]),
        Row::new(vec![
            Cell::from("Base Fee").style(THEME.muted_style()),
            Cell::from(base_fee_str),
            Cell::from("Miner").style(THEME.muted_style()),
            Cell::from(format!("{}", s.miner)).style(THEME.address_style()),
        ]),
    ];

    // ETH Burned row
    if let Some(eth_burned) = s.eth_burned {
        rows.push(Row::new(vec![
            Cell::from("ETH Burned").style(THEME.muted_style()),
            Cell::from(utils::format_eth(eth_burned)).style(THEME.eth_style()),
            Cell::from(""),
            Cell::from(""),
        ]));
    }

    if let Some(size) = detail.size {
        rows.push(Row::new(vec![
            Cell::from("Size").style(THEME.muted_style()),
            Cell::from(format!("{} bytes", utils::format_number(size))),
            Cell::from(""),
            Cell::from(""),
        ]));
    }

    rows
}

fn build_tx_rows(detail: &BlockDetail) -> Vec<Row<'static>> {
    detail
        .transactions
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

impl Component for BlockDetailView {
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
                if let Some(detail) = &self.detail {
                    if let Some(idx) = self.tx_table_state.selected() {
                        if let Some(tx) = detail.transactions.get(idx) {
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
            .title(" Block Detail ")
            .borders(Borders::ALL)
            .border_style(THEME.border_focused_style());

        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        // Show loading state
        if self.loading && self.detail.is_none() {
            let loading = Paragraph::new("Loading...")
                .style(THEME.muted_style())
                .alignment(Alignment::Center);
            let centered = centered_rect(inner, 20, 1);
            frame.render_widget(loading, centered);
            return;
        }

        let detail = match &self.detail {
            Some(d) => d.clone(),
            None => return,
        };

        // Calculate info section height based on number of rows
        let info_row_count = {
            let mut count = 5u16; // base rows
            if detail.summary.eth_burned.is_some() {
                count += 1;
            }
            if detail.size.is_some() {
                count += 1;
            }
            count
        };

        // Split the inner area: info section, gauge, transactions table
        let has_txs = !detail.transactions.is_empty();
        let constraints = if has_txs {
            vec![
                Constraint::Length(info_row_count), // info key-value section
                Constraint::Length(3),               // gas gauge
                Constraint::Min(6),                  // transaction table
            ]
        } else {
            vec![
                Constraint::Length(info_row_count),
                Constraint::Length(3),
                Constraint::Min(1),
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        // -- 1. Block info key-value pairs --
        let info_rows = render_info_section(&detail);
        let info_widths = [
            Constraint::Length(14),
            Constraint::Percentage(35),
            Constraint::Length(14),
            Constraint::Percentage(35),
        ];
        let info_table = Table::new(info_rows, info_widths);
        frame.render_widget(info_table, chunks[0]);

        // -- 2. Gas gauge --
        let gas_pct = utils::gas_utilization_pct(
            detail.summary.gas_used,
            detail.summary.gas_limit,
        );
        let gauge_color = if gas_pct < 50.0 {
            THEME.gas_low
        } else if gas_pct < 80.0 {
            THEME.gas_med
        } else {
            THEME.gas_high
        };

        let gauge_label = format!(
            "Gas: {} / {} ({:.1}%)",
            utils::format_number(detail.summary.gas_used),
            utils::format_number(detail.summary.gas_limit),
            gas_pct
        );

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .title(" Gas Usage ")
                    .borders(Borders::ALL)
                    .border_style(THEME.border_style()),
            )
            .gauge_style(Style::default().fg(gauge_color).bg(THEME.surface))
            .ratio(gas_pct.min(100.0) / 100.0)
            .label(gauge_label);

        frame.render_widget(gauge, chunks[1]);

        // -- 3. Transaction table --
        if has_txs {
            let tx_block = Block::default()
                .title(format!(" Transactions ({}) ", detail.transactions.len()))
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

            let tx_rows = build_tx_rows(&detail);
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

/// Returns a centered rectangle of the given width/height within the parent area.
fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
