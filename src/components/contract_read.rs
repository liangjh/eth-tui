use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::components::Component;
use crate::events::AppEvent;
use crate::theme::THEME;

/// A view/pure function entry parsed from the ABI.
#[derive(Debug, Clone)]
pub struct AbiFunction {
    pub name: String,
    pub inputs: Vec<(String, String)>, // (param_name, param_type)
    pub outputs: Vec<String>,          // type strings
}

pub struct ContractRead {
    pub loading: bool,
    pub address: Option<alloy::primitives::Address>,
    pub functions: Vec<AbiFunction>,
    pub selected: usize,
    pub input_mode: bool,
    pub current_param: usize,
    pub param_inputs: Vec<String>,
    pub result: Option<String>,
    pub error: Option<String>,
    table_state: TableState,
    scroll_state: ScrollbarState,
}

impl ContractRead {
    pub fn new() -> Self {
        Self {
            loading: false,
            address: None,
            functions: Vec::new(),
            selected: 0,
            input_mode: false,
            current_param: 0,
            param_inputs: Vec::new(),
            result: None,
            error: None,
            table_state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::default(),
        }
    }

    /// Set the list of callable functions (view/pure only).
    pub fn set_functions(&mut self, functions: Vec<AbiFunction>) {
        self.functions = functions;
        self.selected = 0;
        self.table_state.select(Some(0));
        self.input_mode = false;
        self.param_inputs.clear();
        self.result = None;
        self.error = None;
    }

    fn select_next(&mut self) {
        if self.functions.is_empty() {
            return;
        }
        let next = if self.selected + 1 >= self.functions.len() {
            self.selected
        } else {
            self.selected + 1
        };
        self.selected = next;
        self.table_state.select(Some(next));
        self.scroll_state = self.scroll_state.position(next);
    }

    fn select_prev(&mut self) {
        if self.functions.is_empty() {
            return;
        }
        let prev = self.selected.saturating_sub(1);
        self.selected = prev;
        self.table_state.select(Some(prev));
        self.scroll_state = self.scroll_state.position(prev);
    }

    fn enter_input_mode(&mut self) {
        if let Some(func) = self.functions.get(self.selected) {
            if func.inputs.is_empty() {
                // No params needed — trigger the call directly
                return;
            }
            self.input_mode = true;
            self.current_param = 0;
            self.param_inputs = vec![String::new(); func.inputs.len()];
            self.result = None;
            self.error = None;
        }
    }

    fn selected_function_name(&self) -> Option<String> {
        self.functions.get(self.selected).map(|f| f.name.clone())
    }
}

impl Component for ContractRead {
    fn handle_key(&mut self, key: KeyEvent) -> Option<AppEvent> {
        if self.input_mode {
            match key.code {
                KeyCode::Esc => {
                    self.input_mode = false;
                    None
                }
                KeyCode::Tab => {
                    if let Some(func) = self.functions.get(self.selected) {
                        if !func.inputs.is_empty() {
                            self.current_param = (self.current_param + 1) % func.inputs.len();
                        }
                    }
                    None
                }
                KeyCode::BackTab => {
                    if let Some(func) = self.functions.get(self.selected) {
                        if !func.inputs.is_empty() {
                            self.current_param = if self.current_param == 0 {
                                func.inputs.len() - 1
                            } else {
                                self.current_param - 1
                            };
                        }
                    }
                    None
                }
                KeyCode::Enter => {
                    // Submit the call
                    self.input_mode = false;
                    self.result = None;
                    self.error = None;
                    self.loading = true;
                    // The app layer will read param_inputs + selected function to make the call
                    None
                }
                KeyCode::Char(c) => {
                    if let Some(input) = self.param_inputs.get_mut(self.current_param) {
                        input.push(c);
                    }
                    None
                }
                KeyCode::Backspace => {
                    if let Some(input) = self.param_inputs.get_mut(self.current_param) {
                        input.pop();
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
                    if !self.functions.is_empty() {
                        self.selected = 0;
                        self.table_state.select(Some(0));
                        self.scroll_state = self.scroll_state.position(0);
                    }
                    None
                }
                KeyCode::Char('G') => {
                    if !self.functions.is_empty() {
                        let last = self.functions.len() - 1;
                        self.selected = last;
                        self.table_state.select(Some(last));
                        self.scroll_state = self.scroll_state.position(last);
                    }
                    None
                }
                KeyCode::Enter => {
                    if self.functions.is_empty() {
                        return None;
                    }
                    let func = &self.functions[self.selected];
                    if func.inputs.is_empty() {
                        // No params — call directly
                        self.loading = true;
                        self.result = None;
                        self.error = None;
                        None
                    } else {
                        self.enter_input_mode();
                        None
                    }
                }
                KeyCode::Esc | KeyCode::Backspace => Some(AppEvent::Back),
                _ => None,
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let outer_block = Block::default()
            .title(" Contract Read ")
            .borders(Borders::ALL)
            .border_style(THEME.border_focused_style());

        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        // Loading with no data
        if self.loading && self.functions.is_empty() && self.address.is_some() {
            let loading = Paragraph::new("Loading ABI...")
                .style(THEME.muted_style())
                .alignment(Alignment::Center);
            frame.render_widget(loading, inner);
            return;
        }

        if self.functions.is_empty() {
            let msg = if self.address.is_some() {
                "No view/pure functions found in ABI"
            } else {
                "No contract selected"
            };
            let text = Paragraph::new(msg)
                .style(THEME.muted_style())
                .alignment(Alignment::Center);
            frame.render_widget(text, inner);
            return;
        }

        // Layout: address header, function list, input area / result
        let has_input = self.input_mode
            || self.result.is_some()
            || self.error.is_some()
            || self.loading;
        let constraints = if has_input {
            vec![
                Constraint::Length(2),  // Address header
                Constraint::Min(6),    // Function list
                Constraint::Length(8), // Input / result area
            ]
        } else {
            vec![
                Constraint::Length(2),
                Constraint::Min(6),
                Constraint::Length(0),
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        // -- Address header --
        if let Some(addr) = self.address {
            let header = Paragraph::new(Line::from(vec![
                Span::styled("  Contract: ", THEME.muted_style()),
                Span::styled(format!("{addr}"), THEME.address_style()),
            ]));
            frame.render_widget(header, chunks[0]);
        }

        // -- Function list table --
        let header = Row::new(vec![
            Cell::from("Function"),
            Cell::from("Inputs"),
            Cell::from("Returns"),
        ])
        .style(THEME.table_header_style())
        .bottom_margin(0);

        let rows: Vec<Row> = self
            .functions
            .iter()
            .map(|f| {
                let inputs = if f.inputs.is_empty() {
                    "()".to_string()
                } else {
                    let params: Vec<String> = f
                        .inputs
                        .iter()
                        .map(|(name, ty)| {
                            if name.is_empty() {
                                ty.clone()
                            } else {
                                format!("{ty} {name}")
                            }
                        })
                        .collect();
                    format!("({})", params.join(", "))
                };
                let outputs = if f.outputs.is_empty() {
                    "void".to_string()
                } else {
                    f.outputs.join(", ")
                };
                Row::new(vec![
                    Cell::from(f.name.clone()).style(THEME.accent_style()),
                    Cell::from(inputs).style(THEME.muted_style()),
                    Cell::from(outputs),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(24),
            Constraint::Min(20),
            Constraint::Length(20),
        ];

        self.scroll_state = self.scroll_state.content_length(self.functions.len());

        let func_block = Block::default()
            .title(format!(" Functions ({}) ", self.functions.len()))
            .borders(Borders::ALL)
            .border_style(THEME.border_style());

        let table = Table::new(rows, widths)
            .header(header)
            .block(func_block)
            .row_highlight_style(THEME.selected_style())
            .highlight_symbol(" > ");

        frame.render_stateful_widget(table, chunks[1], &mut self.table_state);

        // -- Input / Result area --
        if has_input {
            let result_block = Block::default()
                .title(" Call ")
                .borders(Borders::ALL)
                .border_style(THEME.border_style());
            let result_inner = result_block.inner(chunks[2]);
            frame.render_widget(result_block, chunks[2]);

            let mut lines: Vec<Line> = Vec::new();

            if let Some(func) = self.functions.get(self.selected) {
                lines.push(Line::from(vec![
                    Span::styled("  Function: ", THEME.muted_style()),
                    Span::styled(func.name.clone(), THEME.accent_style()),
                ]));

                if self.input_mode && !func.inputs.is_empty() {
                    for (i, (name, ty)) in func.inputs.iter().enumerate() {
                        let label = if name.is_empty() {
                            format!("  {ty}: ")
                        } else {
                            format!("  {name} ({ty}): ")
                        };
                        let value = self
                            .param_inputs
                            .get(i)
                            .cloned()
                            .unwrap_or_default();
                        let cursor = if i == self.current_param { "_" } else { "" };
                        let style = if i == self.current_param {
                            Style::default().fg(THEME.text).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(THEME.text)
                        };
                        lines.push(Line::from(vec![
                            Span::styled(label, THEME.muted_style()),
                            Span::styled(format!("{value}{cursor}"), style),
                        ]));
                    }
                    lines.push(Line::from(Span::styled(
                        "  [Enter] Call  [Tab] Next param  [Esc] Cancel",
                        THEME.muted_style(),
                    )));
                }
            }

            if self.loading {
                lines.push(Line::from(Span::styled(
                    "  Calling...",
                    THEME.muted_style(),
                )));
            }

            if let Some(ref result) = self.result {
                lines.push(Line::from(vec![
                    Span::styled("  Result: ", THEME.muted_style()),
                    Span::styled(
                        result.clone(),
                        Style::default()
                            .fg(THEME.success)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            }

            if let Some(ref err) = self.error {
                lines.push(Line::from(vec![
                    Span::styled("  Error: ", THEME.muted_style()),
                    Span::styled(err.clone(), THEME.error_style()),
                ]));
            }

            let paragraph = Paragraph::new(lines).style(Style::default().fg(THEME.text));
            frame.render_widget(paragraph, result_inner);
        }
    }
}
