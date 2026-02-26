use std::collections::HashMap;

use alloy::primitives::{Address, U256};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::components::Component;
use crate::data::types::WatchEntry;
use crate::events::{AppEvent, View};
use crate::theme::THEME;
use crate::utils;

pub struct WatchListView {
    pub entries: Vec<WatchEntry>,
    pub balances: HashMap<Address, U256>,
    pub selected: usize,
    pub loading: bool,
    pub adding: bool,
    pub input: String,
    pub label_input: String,
    /// Whether we are entering the label (true) or the address (false) in add mode.
    input_stage: AddStage,
    table_state: TableState,
    scroll_state: ScrollbarState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AddStage {
    Address,
    Label,
}

impl WatchListView {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            balances: HashMap::new(),
            selected: 0,
            loading: false,
            adding: false,
            input: String::new(),
            label_input: String::new(),
            input_stage: AddStage::Address,
            table_state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::default(),
        }
    }

    fn select_next(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let next = if self.selected + 1 >= self.entries.len() {
            self.selected
        } else {
            self.selected + 1
        };
        self.selected = next;
        self.table_state.select(Some(next));
        self.scroll_state = self.scroll_state.position(next);
    }

    fn select_prev(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let prev = self.selected.saturating_sub(1);
        self.selected = prev;
        self.table_state.select(Some(prev));
        self.scroll_state = self.scroll_state.position(prev);
    }

    fn select_first(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        self.selected = 0;
        self.table_state.select(Some(0));
        self.scroll_state = self.scroll_state.position(0);
    }

    fn select_last(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let last = self.entries.len() - 1;
        self.selected = last;
        self.table_state.select(Some(last));
        self.scroll_state = self.scroll_state.position(last);
    }
}

impl Component for WatchListView {
    fn handle_key(&mut self, key: KeyEvent) -> Option<AppEvent> {
        if self.adding {
            match key.code {
                KeyCode::Esc => {
                    self.adding = false;
                    self.input.clear();
                    self.label_input.clear();
                    self.input_stage = AddStage::Address;
                    None
                }
                KeyCode::Enter => {
                    match self.input_stage {
                        AddStage::Address => {
                            // Validate address before moving to label
                            if self.input.parse::<Address>().is_ok() {
                                self.input_stage = AddStage::Label;
                            }
                            None
                        }
                        AddStage::Label => {
                            // Submit the new entry
                            if let Ok(addr) = self.input.parse::<Address>() {
                                let label = if self.label_input.is_empty() {
                                    utils::truncate_address(&addr)
                                } else {
                                    self.label_input.clone()
                                };
                                let entry = WatchEntry {
                                    address: addr,
                                    label,
                                    added_at: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs(),
                                };
                                self.entries.push(entry);
                            }
                            self.adding = false;
                            self.input.clear();
                            self.label_input.clear();
                            self.input_stage = AddStage::Address;
                            None
                        }
                    }
                }
                KeyCode::Char(c) => {
                    match self.input_stage {
                        AddStage::Address => self.input.push(c),
                        AddStage::Label => self.label_input.push(c),
                    }
                    None
                }
                KeyCode::Backspace => {
                    match self.input_stage {
                        AddStage::Address => { self.input.pop(); }
                        AddStage::Label => { self.label_input.pop(); }
                    }
                    None
                }
                _ => None,
            }
        } else {
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
                KeyCode::Char('a') => {
                    self.adding = true;
                    self.input.clear();
                    self.label_input.clear();
                    self.input_stage = AddStage::Address;
                    None
                }
                KeyCode::Char('d') => {
                    if !self.entries.is_empty() && self.selected < self.entries.len() {
                        self.entries.remove(self.selected);
                        if self.selected >= self.entries.len() && !self.entries.is_empty() {
                            self.selected = self.entries.len() - 1;
                        }
                        self.table_state.select(Some(self.selected));
                    }
                    None
                }
                KeyCode::Enter => {
                    if let Some(entry) = self.entries.get(self.selected) {
                        return Some(AppEvent::Navigate(View::AddressView(entry.address)));
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
            .title(" Watch List ")
            .borders(Borders::ALL)
            .border_style(THEME.border_focused_style());

        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        if self.entries.is_empty() && !self.adding {
            let text = Paragraph::new(
                "No watched addresses.\n\nPress 'a' to add an address, or press 'w' on any address view.",
            )
            .style(THEME.muted_style())
            .alignment(Alignment::Center);
            frame.render_widget(text, inner);
            return;
        }

        // Layout: table + optional add input area
        let constraints = if self.adding {
            vec![Constraint::Min(4), Constraint::Length(4)]
        } else {
            vec![Constraint::Min(4), Constraint::Length(0)]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        // -- Watch list table --
        let header = Row::new(vec![
            Cell::from("#"),
            Cell::from("Label"),
            Cell::from("Address"),
            Cell::from("Balance"),
            Cell::from("Added"),
        ])
        .style(THEME.table_header_style())
        .bottom_margin(0);

        let rows: Vec<Row> = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let balance = self
                    .balances
                    .get(&entry.address)
                    .map(|b| utils::format_eth(*b))
                    .unwrap_or_else(|| "...".to_string());
                let time = utils::format_time_ago(entry.added_at);

                Row::new(vec![
                    Cell::from(format!("{}", i + 1)),
                    Cell::from(entry.label.clone()).style(THEME.accent_style()),
                    Cell::from(utils::truncate_address(&entry.address)).style(THEME.address_style()),
                    Cell::from(balance).style(THEME.eth_style()),
                    Cell::from(time).style(THEME.muted_style()),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(4),
            Constraint::Length(16),
            Constraint::Length(15),
            Constraint::Length(20),
            Constraint::Min(10),
        ];

        self.scroll_state = self.scroll_state.content_length(self.entries.len());

        let table_block = Block::default().borders(Borders::NONE);
        let table = Table::new(rows, widths)
            .header(header)
            .block(table_block)
            .row_highlight_style(THEME.selected_style())
            .highlight_symbol(" > ");

        frame.render_stateful_widget(table, chunks[0], &mut self.table_state);

        // -- Add input area --
        if self.adding {
            let add_block = Block::default()
                .title(" Add Address ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(THEME.warning));
            let add_inner = add_block.inner(chunks[1]);
            frame.render_widget(add_block, chunks[1]);

            let (addr_cursor, label_cursor) = match self.input_stage {
                AddStage::Address => ("_", ""),
                AddStage::Label => ("", "_"),
            };

            let lines = vec![
                Line::from(vec![
                    Span::styled("  Address: ", THEME.muted_style()),
                    Span::styled(
                        format!("{}{}", self.input, addr_cursor),
                        if self.input_stage == AddStage::Address {
                            Style::default().fg(THEME.text).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(THEME.text)
                        },
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  Label:   ", THEME.muted_style()),
                    Span::styled(
                        format!("{}{}", self.label_input, label_cursor),
                        if self.input_stage == AddStage::Label {
                            Style::default().fg(THEME.text).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(THEME.text)
                        },
                    ),
                ]),
            ];

            let paragraph = Paragraph::new(lines);
            frame.render_widget(paragraph, add_inner);
        }

        // Scrollbar
        if self.entries.len() > inner.height as usize {
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
