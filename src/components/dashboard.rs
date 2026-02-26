use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::components::Component;
use crate::data::types::{BlockSummary, TransactionSummary};
use crate::events::{AppEvent, View};
use crate::theme::THEME;
use crate::utils;

#[derive(Debug, Clone, Copy, PartialEq)]
enum DashboardPanel {
    Blocks,
    Transactions,
}

pub struct Dashboard {
    pub blocks: Vec<BlockSummary>,
    pub transactions: Vec<TransactionSummary>,
    active_panel: DashboardPanel,
    block_state: TableState,
    tx_state: TableState,
}

impl Dashboard {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            transactions: Vec::new(),
            active_panel: DashboardPanel::Blocks,
            block_state: TableState::default(),
            tx_state: TableState::default(),
        }
    }

    fn active_table_len(&self) -> usize {
        match self.active_panel {
            DashboardPanel::Blocks => self.blocks.len(),
            DashboardPanel::Transactions => self.transactions.len(),
        }
    }

    fn active_state_mut(&mut self) -> &mut TableState {
        match self.active_panel {
            DashboardPanel::Blocks => &mut self.block_state,
            DashboardPanel::Transactions => &mut self.tx_state,
        }
    }

    fn select_next(&mut self) {
        let len = self.active_table_len();
        if len == 0 {
            return;
        }
        let state = self.active_state_mut();
        let current = state.selected().unwrap_or(0);
        let next = if current + 1 >= len { current } else { current + 1 };
        state.select(Some(next));
    }

    fn select_prev(&mut self) {
        let len = self.active_table_len();
        if len == 0 {
            return;
        }
        let state = self.active_state_mut();
        let current = state.selected().unwrap_or(0);
        let prev = current.saturating_sub(1);
        state.select(Some(prev));
    }

    fn select_first(&mut self) {
        let len = self.active_table_len();
        if len == 0 {
            return;
        }
        self.active_state_mut().select(Some(0));
    }

    fn select_last(&mut self) {
        let len = self.active_table_len();
        if len == 0 {
            return;
        }
        self.active_state_mut().select(Some(len - 1));
    }
}

fn build_block_rows(blocks: &[BlockSummary]) -> Vec<Row<'static>> {
    blocks
        .iter()
        .map(|b| {
            let gas_pct = utils::gas_utilization_pct(b.gas_used, b.gas_limit);
            Row::new(vec![
                Cell::from(format!("{}", b.number)).style(THEME.accent_style()),
                Cell::from(utils::format_time_ago(b.timestamp)).style(THEME.muted_style()),
                Cell::from(format!("{}", b.tx_count)),
                Cell::from(format!("{:.1}%", gas_pct)).style(THEME.gas_style(gas_pct)),
                Cell::from(utils::truncate_address(&b.miner)).style(THEME.address_style()),
            ])
        })
        .collect()
}

fn build_tx_rows(transactions: &[TransactionSummary]) -> Vec<Row<'static>> {
    transactions
        .iter()
        .map(|tx| {
            let to_str = tx
                .to
                .as_ref()
                .map(|a| utils::truncate_address(a))
                .unwrap_or_else(|| "Contract".to_string());
            let from_to = format!("{}  {}", utils::truncate_address(&tx.from), to_str);
            let method_display = tx
                .method_name
                .clone()
                .or_else(|| tx.method_id.as_ref().map(|id| utils::format_selector(id)))
                .unwrap_or_else(|| "Transfer".to_string());
            Row::new(vec![
                Cell::from(utils::truncate_hash(&tx.hash)).style(THEME.hash_style()),
                Cell::from(from_to).style(THEME.address_style()),
                Cell::from(utils::format_eth(tx.value)).style(THEME.eth_style()),
                Cell::from(method_display).style(THEME.muted_style()),
            ])
        })
        .collect()
}

impl Component for Dashboard {
    fn handle_key(&mut self, key: KeyEvent) -> Option<AppEvent> {
        match key.code {
            KeyCode::Tab => {
                self.active_panel = match self.active_panel {
                    DashboardPanel::Blocks => DashboardPanel::Transactions,
                    DashboardPanel::Transactions => DashboardPanel::Blocks,
                };
                None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.select_next();
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.select_prev();
                None
            }
            KeyCode::Char('g') => {
                self.select_first();
                None
            }
            KeyCode::Char('G') => {
                self.select_last();
                None
            }
            KeyCode::Enter => match self.active_panel {
                DashboardPanel::Blocks => {
                    if let Some(idx) = self.block_state.selected() {
                        if let Some(block) = self.blocks.get(idx) {
                            return Some(AppEvent::Navigate(View::BlockDetail(block.number)));
                        }
                    }
                    None
                }
                DashboardPanel::Transactions => {
                    if let Some(idx) = self.tx_state.selected() {
                        if let Some(tx) = self.transactions.get(idx) {
                            return Some(AppEvent::Navigate(View::TransactionDetail(tx.hash)));
                        }
                    }
                    None
                }
            },
            _ => None,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // --- Left panel: Recent Blocks ---
        let block_border_style = if self.active_panel == DashboardPanel::Blocks {
            THEME.border_focused_style()
        } else {
            THEME.border_style()
        };
        let block_block = Block::default()
            .title(" Recent Blocks ")
            .borders(Borders::ALL)
            .border_style(block_border_style);

        let block_header = Row::new(vec![
            Cell::from("Block #"),
            Cell::from("Time"),
            Cell::from("Txns"),
            Cell::from("Gas Used %"),
            Cell::from("Miner"),
        ])
        .style(THEME.table_header_style())
        .bottom_margin(0);

        let block_rows = build_block_rows(&self.blocks);
        let block_widths = [
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(6),
            Constraint::Length(10),
            Constraint::Min(14),
        ];

        let block_table = Table::new(block_rows, block_widths)
            .header(block_header)
            .block(block_block)
            .row_highlight_style(THEME.selected_style())
            .highlight_symbol(" > ");

        frame.render_stateful_widget(block_table, chunks[0], &mut self.block_state);

        // --- Right panel: Recent Transactions ---
        let tx_border_style = if self.active_panel == DashboardPanel::Transactions {
            THEME.border_focused_style()
        } else {
            THEME.border_style()
        };
        let tx_block = Block::default()
            .title(" Recent Transactions ")
            .borders(Borders::ALL)
            .border_style(tx_border_style);

        let tx_header = Row::new(vec![
            Cell::from("Hash"),
            Cell::from("From / To"),
            Cell::from("Value"),
            Cell::from("Method"),
        ])
        .style(THEME.table_header_style())
        .bottom_margin(0);

        let tx_rows = build_tx_rows(&self.transactions);
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

        frame.render_stateful_widget(tx_table, chunks[1], &mut self.tx_state);
    }
}
