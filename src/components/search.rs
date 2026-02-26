use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::theme::THEME;

pub struct SearchBar {
    pub active: bool,
    pub input: String,
    cursor_position: usize,
    pub error: Option<String>,
}

impl SearchBar {
    pub fn new() -> Self {
        Self {
            active: false,
            input: String::new(),
            cursor_position: 0,
            error: None,
        }
    }

    pub fn activate(&mut self) {
        self.active = true;
        self.input.clear();
        self.cursor_position = 0;
        self.error = None;
    }

    pub fn deactivate(&mut self) {
        self.active = false;
        self.error = None;
    }

    /// Returns Some(query) if the user pressed Enter, None otherwise.
    /// Returns Some("") if Esc was pressed (caller should deactivate).
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<String> {
        if !self.active {
            return None;
        }

        match key.code {
            KeyCode::Enter => {
                let query = self.input.clone();
                self.active = false;
                Some(query)
            }
            KeyCode::Esc => {
                self.deactivate();
                Some(String::new())
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    self.input.remove(self.cursor_position);
                }
                self.error = None;
                None
            }
            KeyCode::Delete => {
                if self.cursor_position < self.input.len() {
                    self.input.remove(self.cursor_position);
                }
                self.error = None;
                None
            }
            KeyCode::Left => {
                self.cursor_position = self.cursor_position.saturating_sub(1);
                None
            }
            KeyCode::Right => {
                if self.cursor_position < self.input.len() {
                    self.cursor_position += 1;
                }
                None
            }
            KeyCode::Home => {
                self.cursor_position = 0;
                None
            }
            KeyCode::End => {
                self.cursor_position = self.input.len();
                None
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'u' {
                    self.input.clear();
                    self.cursor_position = 0;
                } else {
                    self.input.insert(self.cursor_position, c);
                    self.cursor_position += 1;
                }
                self.error = None;
                None
            }
            _ => None,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.active {
            return;
        }

        let width = area.width.min(70);
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let popup_area = Rect::new(x, area.y + 2, width, 3);

        frame.render_widget(Clear, popup_area);

        let border_style = if self.error.is_some() {
            Style::default().fg(THEME.error)
        } else {
            THEME.border_focused_style()
        };

        let title = if let Some(ref err) = self.error {
            format!(" Search - {err} ")
        } else {
            " Search (address / tx hash / block #) ".to_string()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title)
            .style(Style::default().bg(THEME.surface));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        let display_text = if self.input.is_empty() {
            Span::styled("Type to search...", THEME.muted_style())
        } else {
            Span::styled(&self.input, Style::default().fg(THEME.text))
        };

        let input_paragraph = Paragraph::new(display_text);
        frame.render_widget(input_paragraph, inner);

        let cursor_x = inner.x + self.cursor_position as u16;
        let cursor_y = inner.y;
        if cursor_x < inner.right() {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}
