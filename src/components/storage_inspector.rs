use alloy::primitives::{Address, B256, U256};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::components::Component;
use crate::events::AppEvent;
use crate::theme::THEME;

pub struct StorageInspector {
    pub address: Option<Address>,
    pub slot_input: String,
    pub results: Vec<(U256, B256)>,
    pub input_mode: bool,
    pub loading: bool,
    selected: usize,
    table_state: TableState,
    scroll_state: ScrollbarState,
}

impl StorageInspector {
    pub fn new() -> Self {
        Self {
            address: None,
            slot_input: String::new(),
            results: Vec::new(),
            input_mode: false,
            loading: false,
            selected: 0,
            table_state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::default(),
        }
    }

    /// Add a storage result to the table.
    pub fn add_result(&mut self, slot: U256, value: B256) {
        // Replace if same slot already queried
        if let Some(existing) = self.results.iter_mut().find(|(s, _)| *s == slot) {
            existing.1 = value;
        } else {
            self.results.push((slot, value));
        }
        self.loading = false;
    }

    fn select_next(&mut self) {
        if self.results.is_empty() {
            return;
        }
        let next = if self.selected + 1 >= self.results.len() {
            self.selected
        } else {
            self.selected + 1
        };
        self.selected = next;
        self.table_state.select(Some(next));
        self.scroll_state = self.scroll_state.position(next);
    }

    fn select_prev(&mut self) {
        if self.results.is_empty() {
            return;
        }
        let prev = self.selected.saturating_sub(1);
        self.selected = prev;
        self.table_state.select(Some(prev));
        self.scroll_state = self.scroll_state.position(prev);
    }
}

impl Component for StorageInspector {
    fn handle_key(&mut self, key: KeyEvent) -> Option<AppEvent> {
        if self.input_mode {
            match key.code {
                KeyCode::Esc => {
                    self.input_mode = false;
                    None
                }
                KeyCode::Enter => {
                    // Parse the slot and trigger a query
                    self.input_mode = false;
                    self.loading = true;
                    // The app layer reads slot_input to make the RPC call
                    None
                }
                KeyCode::Char(c) => {
                    // Allow hex digits and 'x' prefix
                    if c.is_ascii_hexdigit() || c == 'x' || c == 'X' {
                        self.slot_input.push(c);
                    }
                    None
                }
                KeyCode::Backspace => {
                    self.slot_input.pop();
                    None
                }
                _ => None,
            }
        } else {
            match key.code {
                KeyCode::Char('i') => {
                    self.input_mode = true;
                    self.slot_input.clear();
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
                    if !self.results.is_empty() {
                        self.selected = 0;
                        self.table_state.select(Some(0));
                        self.scroll_state = self.scroll_state.position(0);
                    }
                    None
                }
                KeyCode::Char('G') => {
                    if !self.results.is_empty() {
                        let last = self.results.len() - 1;
                        self.selected = last;
                        self.table_state.select(Some(last));
                        self.scroll_state = self.scroll_state.position(last);
                    }
                    None
                }
                KeyCode::Esc | KeyCode::Backspace => Some(AppEvent::Back),
                _ => None,
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let outer_block = Block::default()
            .title(" Storage Inspector ")
            .borders(Borders::ALL)
            .border_style(THEME.border_focused_style());

        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        // Layout: address header + input area, results table
        let constraints = vec![
            Constraint::Length(5), // Header + input
            Constraint::Min(4),   // Results table
        ];

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        // -- Header + input --
        let mut header_lines: Vec<Line> = Vec::new();

        if let Some(addr) = self.address {
            header_lines.push(Line::from(vec![
                Span::styled("  Address: ", THEME.muted_style()),
                Span::styled(format!("{addr}"), THEME.address_style()),
            ]));
        } else {
            header_lines.push(Line::from(Span::styled(
                "  No address selected",
                THEME.muted_style(),
            )));
        }

        header_lines.push(Line::from(""));

        if self.input_mode {
            let cursor = "_";
            header_lines.push(Line::from(vec![
                Span::styled("  Slot: ", THEME.muted_style()),
                Span::styled(
                    format!("{}{cursor}", self.slot_input),
                    Style::default()
                        .fg(THEME.text)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            header_lines.push(Line::from(Span::styled(
                "  Enter slot number (decimal or 0x hex). [Enter] Query  [Esc] Cancel",
                THEME.muted_style(),
            )));
        } else if self.loading {
            header_lines.push(Line::from(Span::styled(
                "  Querying storage...",
                THEME.muted_style(),
            )));
        } else {
            header_lines.push(Line::from(Span::styled(
                "  Press 'i' to enter a storage slot number",
                THEME.muted_style(),
            )));
        }

        let header_paragraph =
            Paragraph::new(header_lines).style(Style::default().fg(THEME.text));
        frame.render_widget(header_paragraph, chunks[0]);

        // -- Results table --
        if self.results.is_empty() {
            let empty_msg = Paragraph::new("  No storage slots queried yet")
                .style(THEME.muted_style());
            frame.render_widget(empty_msg, chunks[1]);
            return;
        }

        let table_block = Block::default()
            .title(format!(" Results ({}) ", self.results.len()))
            .borders(Borders::ALL)
            .border_style(THEME.border_style());

        let header = Row::new(vec![
            Cell::from("#"),
            Cell::from("Slot"),
            Cell::from("Value (hex)"),
            Cell::from("Value (dec)"),
        ])
        .style(THEME.table_header_style())
        .bottom_margin(0);

        let rows: Vec<Row> = self
            .results
            .iter()
            .enumerate()
            .map(|(i, (slot, value))| {
                let slot_hex = format!("{slot:#x}");
                let slot_display = if slot_hex.len() > 20 {
                    format!("{}...{}", &slot_hex[..10], &slot_hex[slot_hex.len() - 6..])
                } else {
                    slot_hex
                };

                let value_hex = format!("{value}");
                let value_display = if value_hex.len() > 34 {
                    format!("{}...{}", &value_hex[..18], &value_hex[value_hex.len() - 8..])
                } else {
                    value_hex
                };

                // Try to show decimal for small values
                let value_u256 = U256::from_be_bytes(value.0);
                let dec_display = if value_u256 < U256::from(1u64 << 53) {
                    format!("{value_u256}")
                } else {
                    "large".to_string()
                };

                Row::new(vec![
                    Cell::from(format!("{}", i + 1)),
                    Cell::from(slot_display).style(THEME.accent_style()),
                    Cell::from(value_display).style(THEME.hash_style()),
                    Cell::from(dec_display),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(4),
            Constraint::Length(22),
            Constraint::Min(24),
            Constraint::Length(16),
        ];

        self.scroll_state = self.scroll_state.content_length(self.results.len());

        let table = Table::new(rows, widths)
            .header(header)
            .block(table_block)
            .row_highlight_style(THEME.selected_style())
            .highlight_symbol(" > ");

        frame.render_stateful_widget(table, chunks[1], &mut self.table_state);

        // Scrollbar
        if self.results.len() > chunks[1].height as usize {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("^"))
                .end_symbol(Some("v"));

            let scrollbar_area = Rect {
                x: chunks[1].x + chunks[1].width.saturating_sub(1),
                y: chunks[1].y + 1,
                width: 1,
                height: chunks[1].height.saturating_sub(2),
            };

            frame.render_stateful_widget(scrollbar, scrollbar_area, &mut self.scroll_state);
        }
    }
}
