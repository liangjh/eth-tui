use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::components::Component;
use crate::data::types::ExecutionTrace;
use crate::events::AppEvent;
use crate::theme::THEME;
use crate::utils;

/// Opcodes that get special highlighting.
const CALL_OPS: &[&str] = &["CALL", "CALLCODE", "DELEGATECALL", "STATICCALL"];
const CREATE_OPS: &[&str] = &["CREATE", "CREATE2"];

pub struct TxDebugger {
    pub trace: Option<ExecutionTrace>,
    pub current_step: usize,
    pub loading: bool,
    table_state: TableState,
    scroll_state: ScrollbarState,
}

impl TxDebugger {
    pub fn new() -> Self {
        Self {
            trace: None,
            current_step: 0,
            loading: false,
            table_state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::default(),
        }
    }

    fn step_count(&self) -> usize {
        self.trace.as_ref().map(|t| t.steps.len()).unwrap_or(0)
    }

    fn select_next(&mut self) {
        let len = self.step_count();
        if len == 0 {
            return;
        }
        let next = if self.current_step + 1 >= len {
            self.current_step
        } else {
            self.current_step + 1
        };
        self.current_step = next;
        self.table_state.select(Some(next));
        self.scroll_state = self.scroll_state.position(next);
    }

    fn select_prev(&mut self) {
        if self.step_count() == 0 {
            return;
        }
        let prev = self.current_step.saturating_sub(1);
        self.current_step = prev;
        self.table_state.select(Some(prev));
        self.scroll_state = self.scroll_state.position(prev);
    }

    fn select_first(&mut self) {
        if self.step_count() == 0 {
            return;
        }
        self.current_step = 0;
        self.table_state.select(Some(0));
        self.scroll_state = self.scroll_state.position(0);
    }

    fn select_last(&mut self) {
        let len = self.step_count();
        if len == 0 {
            return;
        }
        let last = len - 1;
        self.current_step = last;
        self.table_state.select(Some(last));
        self.scroll_state = self.scroll_state.position(last);
    }

    fn page_down(&mut self) {
        let len = self.step_count();
        if len == 0 {
            return;
        }
        let last = len - 1;
        self.current_step = (self.current_step + 20).min(last);
        self.table_state.select(Some(self.current_step));
        self.scroll_state = self.scroll_state.position(self.current_step);
    }

    fn page_up(&mut self) {
        if self.step_count() == 0 {
            return;
        }
        self.current_step = self.current_step.saturating_sub(20);
        self.table_state.select(Some(self.current_step));
        self.scroll_state = self.scroll_state.position(self.current_step);
    }

    fn op_style(op: &str) -> Style {
        if CALL_OPS.contains(&op) {
            Style::default()
                .fg(THEME.info)
                .add_modifier(Modifier::BOLD)
        } else if CREATE_OPS.contains(&op) {
            Style::default()
                .fg(THEME.warning)
                .add_modifier(Modifier::BOLD)
        } else if op == "REVERT" || op == "INVALID" {
            Style::default()
                .fg(THEME.error)
                .add_modifier(Modifier::BOLD)
        } else if op == "RETURN" || op == "STOP" {
            Style::default().fg(THEME.success)
        } else {
            Style::default().fg(THEME.text)
        }
    }
}

impl Component for TxDebugger {
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
            (KeyCode::Esc, _) | (KeyCode::Backspace, _) => Some(AppEvent::Back),
            _ => None,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let outer_block = Block::default()
            .title(" Transaction Debugger ")
            .borders(Borders::ALL)
            .border_style(THEME.border_focused_style());

        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        // Loading state
        if self.loading && self.trace.is_none() {
            let text = Paragraph::new("Loading execution trace...")
                .style(THEME.muted_style())
                .alignment(Alignment::Center);
            frame.render_widget(text, inner);
            return;
        }

        let trace = match &self.trace {
            Some(t) => t,
            None => {
                let text = Paragraph::new("No trace data available")
                    .style(THEME.muted_style())
                    .alignment(Alignment::Center);
                frame.render_widget(text, inner);
                return;
            }
        };

        if trace.steps.is_empty() {
            let text = Paragraph::new("Trace has no execution steps")
                .style(THEME.muted_style())
                .alignment(Alignment::Center);
            frame.render_widget(text, inner);
            return;
        }

        // Layout: opcode table (left) | stack display (right)
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(inner);

        // -- Left: Opcode table --
        let step_count = trace.steps.len();
        let title = format!(
            " Steps ({}/{}) | Gas Used: {} ",
            self.current_step + 1,
            step_count,
            utils::format_number(trace.gas_used),
        );

        let table_block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(THEME.border_style());

        let header = Row::new(vec![
            Cell::from("Step"),
            Cell::from("PC"),
            Cell::from("Opcode"),
            Cell::from("Gas"),
            Cell::from("Cost"),
            Cell::from("Depth"),
        ])
        .style(THEME.table_header_style())
        .bottom_margin(0);

        let rows: Vec<Row> = trace
            .steps
            .iter()
            .enumerate()
            .map(|(i, step)| {
                let op_style = Self::op_style(&step.op);
                let depth_indent = "  ".repeat(step.depth.saturating_sub(1));
                let has_error = step.error.is_some();

                let mut row = Row::new(vec![
                    Cell::from(format!("{}", i)),
                    Cell::from(format!("{}", step.pc)),
                    Cell::from(format!("{}{}", depth_indent, step.op)).style(op_style),
                    Cell::from(utils::format_number(step.gas)),
                    Cell::from(utils::format_number(step.gas_cost)),
                    Cell::from(format!("{}", step.depth)),
                ]);

                if has_error {
                    row = row.style(Style::default().bg(Color::Rgb(60, 20, 20)));
                }

                row
            })
            .collect();

        let widths = [
            Constraint::Length(7),
            Constraint::Length(7),
            Constraint::Min(16),
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Length(6),
        ];

        self.scroll_state = self.scroll_state.content_length(step_count);

        let table = Table::new(rows, widths)
            .header(header)
            .block(table_block)
            .row_highlight_style(THEME.selected_style())
            .highlight_symbol(" > ");

        frame.render_stateful_widget(table, h_chunks[0], &mut self.table_state);

        // -- Right: Stack display --
        let stack_block = Block::default()
            .title(" Stack ")
            .borders(Borders::ALL)
            .border_style(THEME.border_style());
        let stack_inner = stack_block.inner(h_chunks[1]);
        frame.render_widget(stack_block, h_chunks[1]);

        let current = &trace.steps[self.current_step];
        let mut stack_lines: Vec<Line> = Vec::new();

        // Show current step info
        stack_lines.push(Line::from(vec![
            Span::styled("  Op: ", THEME.muted_style()),
            Span::styled(current.op.clone(), Self::op_style(&current.op)),
        ]));
        stack_lines.push(Line::from(vec![
            Span::styled("  PC: ", THEME.muted_style()),
            Span::raw(format!("{}", current.pc)),
        ]));

        if let Some(ref err) = current.error {
            stack_lines.push(Line::from(vec![
                Span::styled("  Err: ", Style::default().fg(THEME.error)),
                Span::styled(err.clone(), THEME.error_style()),
            ]));
        }

        stack_lines.push(Line::from(""));
        stack_lines.push(Line::from(Span::styled(
            "  Stack (top 8):",
            Style::default()
                .fg(THEME.text)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )));
        stack_lines.push(Line::from(""));

        // Show top 8 stack items (stack is stored top-first)
        let stack_items: Vec<_> = current.stack.iter().rev().take(8).collect();
        if stack_items.is_empty() {
            stack_lines.push(Line::from(Span::styled(
                "  (empty)",
                THEME.muted_style(),
            )));
        } else {
            for (i, val) in stack_items.iter().enumerate() {
                let hex_str = format!("{val:#x}");
                let display = if hex_str.len() > 32 {
                    format!("{}...{}", &hex_str[..16], &hex_str[hex_str.len() - 8..])
                } else {
                    hex_str
                };
                stack_lines.push(Line::from(vec![
                    Span::styled(format!("  [{i}] "), THEME.muted_style()),
                    Span::styled(display, THEME.hash_style()),
                ]));
            }
        }

        let stack_paragraph =
            Paragraph::new(stack_lines).style(Style::default().fg(THEME.text));
        frame.render_widget(stack_paragraph, stack_inner);

        // Scrollbar for the opcode table
        if step_count > h_chunks[0].height as usize {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("^"))
                .end_symbol(Some("v"));

            let scrollbar_area = Rect {
                x: h_chunks[0].x + h_chunks[0].width.saturating_sub(1),
                y: h_chunks[0].y + 1,
                width: 1,
                height: h_chunks[0].height.saturating_sub(2),
            };

            frame.render_stateful_widget(scrollbar, scrollbar_area, &mut self.scroll_state);
        }
    }
}
