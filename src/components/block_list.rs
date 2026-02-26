use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::components::Component;
use crate::data::types::BlockSummary;
use crate::events::{AppEvent, View};
use crate::theme::THEME;
use crate::utils;

pub struct BlockList {
    pub blocks: Vec<BlockSummary>,
    table_state: TableState,
    scroll_state: ScrollbarState,
}

impl BlockList {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            table_state: TableState::default(),
            scroll_state: ScrollbarState::default(),
        }
    }

    fn select_next(&mut self) {
        let len = self.blocks.len();
        if len == 0 {
            return;
        }
        let current = self.table_state.selected().unwrap_or(0);
        let next = if current + 1 >= len { current } else { current + 1 };
        self.table_state.select(Some(next));
        self.scroll_state = self.scroll_state.position(next);
    }

    fn select_prev(&mut self) {
        let len = self.blocks.len();
        if len == 0 {
            return;
        }
        let current = self.table_state.selected().unwrap_or(0);
        let prev = current.saturating_sub(1);
        self.table_state.select(Some(prev));
        self.scroll_state = self.scroll_state.position(prev);
    }

    fn select_first(&mut self) {
        if self.blocks.is_empty() {
            return;
        }
        self.table_state.select(Some(0));
        self.scroll_state = self.scroll_state.position(0);
    }

    fn select_last(&mut self) {
        let len = self.blocks.len();
        if len == 0 {
            return;
        }
        self.table_state.select(Some(len - 1));
        self.scroll_state = self.scroll_state.position(len - 1);
    }
}

fn build_rows(blocks: &[BlockSummary]) -> Vec<Row<'static>> {
    blocks
        .iter()
        .map(|b| {
            let gas_pct = utils::gas_utilization_pct(b.gas_used, b.gas_limit);
            let base_fee_str = b
                .base_fee
                .map(|fee| utils::format_gwei(fee))
                .unwrap_or_else(|| "N/A".to_string());

            Row::new(vec![
                Cell::from(format!("{}", b.number)).style(THEME.accent_style()),
                Cell::from(utils::truncate_hash(&b.hash)).style(THEME.hash_style()),
                Cell::from(utils::format_time_ago(b.timestamp)).style(THEME.muted_style()),
                Cell::from(format!("{}", b.tx_count)),
                Cell::from(utils::format_number(b.gas_used)),
                Cell::from(format!("{:.1}%", gas_pct)).style(THEME.gas_style(gas_pct)),
                Cell::from(base_fee_str),
                Cell::from(utils::truncate_address(&b.miner)).style(THEME.address_style()),
            ])
        })
        .collect()
}

impl Component for BlockList {
    fn handle_key(&mut self, key: KeyEvent) -> Option<AppEvent> {
        match key.code {
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
            KeyCode::Enter => {
                if let Some(idx) = self.table_state.selected() {
                    if let Some(block) = self.blocks.get(idx) {
                        return Some(AppEvent::Navigate(View::BlockDetail(block.number)));
                    }
                }
                None
            }
            KeyCode::Esc | KeyCode::Backspace => Some(AppEvent::Back),
            _ => None,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let outer_block = Block::default()
            .title(" Blocks ")
            .borders(Borders::ALL)
            .border_style(THEME.border_focused_style());

        let header = Row::new(vec![
            Cell::from("Block #"),
            Cell::from("Hash"),
            Cell::from("Time"),
            Cell::from("Txns"),
            Cell::from("Gas Used"),
            Cell::from("Gas %"),
            Cell::from("Base Fee"),
            Cell::from("Miner"),
        ])
        .style(THEME.table_header_style())
        .bottom_margin(0);

        let rows = build_rows(&self.blocks);
        let widths = [
            Constraint::Length(10),
            Constraint::Length(14),
            Constraint::Length(8),
            Constraint::Length(6),
            Constraint::Length(14),
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Min(14),
        ];

        // Update scroll state content length
        self.scroll_state = self.scroll_state.content_length(self.blocks.len());

        let table = Table::new(rows, widths)
            .header(header)
            .block(outer_block)
            .row_highlight_style(THEME.selected_style())
            .highlight_symbol(" > ");

        frame.render_stateful_widget(table, area, &mut self.table_state);

        // Render scrollbar on the right side
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("^"))
            .end_symbol(Some("v"));

        // Scrollbar area is slightly inset from the table border
        let scrollbar_area = Rect {
            x: area.x + area.width.saturating_sub(1),
            y: area.y + 1,
            width: 1,
            height: area.height.saturating_sub(2),
        };

        frame.render_stateful_widget(scrollbar, scrollbar_area, &mut self.scroll_state);
    }
}
