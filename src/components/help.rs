use crossterm::event::KeyEvent;
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::theme::THEME;

pub struct HelpOverlay {
    pub visible: bool,
}

impl HelpOverlay {
    pub fn new() -> Self {
        Self { visible: false }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Returns true if it consumed the event
    pub fn handle_key(&mut self, _key: KeyEvent) -> bool {
        if self.visible {
            self.visible = false;
            true
        } else {
            false
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let popup_width = area.width * 60 / 100;
        let popup_height = area.height * 70 / 100;
        let x = area.x + (area.width - popup_width) / 2;
        let y = area.y + (area.height - popup_height) / 2;
        let popup_area = Rect::new(x, y, popup_width, popup_height);

        // Clear the area behind the popup
        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .title(" Keyboard Shortcuts ")
            .borders(Borders::ALL)
            .border_style(THEME.border_focused_style())
            .style(Style::default().bg(THEME.surface));

        let help_text = vec![
            Line::from(Span::styled(
                "Navigation",
                Style::default()
                    .fg(THEME.text_accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled(
                    "  \u{2191}/k      ",
                    Style::default().fg(THEME.text_accent),
                ),
                Span::styled("Move up", Style::default().fg(THEME.text)),
            ]),
            Line::from(vec![
                Span::styled(
                    "  \u{2193}/j      ",
                    Style::default().fg(THEME.text_accent),
                ),
                Span::styled("Move down", Style::default().fg(THEME.text)),
            ]),
            Line::from(vec![
                Span::styled("  Enter    ", Style::default().fg(THEME.text_accent)),
                Span::styled("Select / Open detail", Style::default().fg(THEME.text)),
            ]),
            Line::from(vec![
                Span::styled("  Esc      ", Style::default().fg(THEME.text_accent)),
                Span::styled("Go back / Close", Style::default().fg(THEME.text)),
            ]),
            Line::from(vec![
                Span::styled("  Tab      ", Style::default().fg(THEME.text_accent)),
                Span::styled("Switch panel", Style::default().fg(THEME.text)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Search",
                Style::default()
                    .fg(THEME.text_accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled("  /        ", Style::default().fg(THEME.text_accent)),
                Span::styled("Open search", Style::default().fg(THEME.text)),
            ]),
            Line::from(vec![
                Span::styled("  Enter    ", Style::default().fg(THEME.text_accent)),
                Span::styled("Submit search", Style::default().fg(THEME.text)),
            ]),
            Line::from(vec![
                Span::styled("  Esc      ", Style::default().fg(THEME.text_accent)),
                Span::styled("Cancel search", Style::default().fg(THEME.text)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Views",
                Style::default()
                    .fg(THEME.text_accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled("  1        ", Style::default().fg(THEME.text_accent)),
                Span::styled("Dashboard", Style::default().fg(THEME.text)),
            ]),
            Line::from(vec![
                Span::styled("  2        ", Style::default().fg(THEME.text_accent)),
                Span::styled("Blocks", Style::default().fg(THEME.text)),
            ]),
            Line::from(vec![
                Span::styled("  3        ", Style::default().fg(THEME.text_accent)),
                Span::styled("Gas Tracker", Style::default().fg(THEME.text)),
            ]),
            Line::from(vec![
                Span::styled("  4        ", Style::default().fg(THEME.text_accent)),
                Span::styled("Watch List", Style::default().fg(THEME.text)),
            ]),
            Line::from(vec![
                Span::styled("  5        ", Style::default().fg(THEME.text_accent)),
                Span::styled("Mempool", Style::default().fg(THEME.text)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Context Actions",
                Style::default()
                    .fg(THEME.text_accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled("  w        ", Style::default().fg(THEME.text_accent)),
                Span::styled("Add to Watchlist (address view)", Style::default().fg(THEME.text)),
            ]),
            Line::from(vec![
                Span::styled("  e        ", Style::default().fg(THEME.text_accent)),
                Span::styled("Export current view data", Style::default().fg(THEME.text)),
            ]),
            Line::from(vec![
                Span::styled("  r        ", Style::default().fg(THEME.text_accent)),
                Span::styled("Contract Read (address view)", Style::default().fg(THEME.text)),
            ]),
            Line::from(vec![
                Span::styled("  d        ", Style::default().fg(THEME.text_accent)),
                Span::styled("Debug Trace (tx view)", Style::default().fg(THEME.text)),
            ]),
            Line::from(vec![
                Span::styled("  S        ", Style::default().fg(THEME.text_accent)),
                Span::styled("Storage Inspector (address view)", Style::default().fg(THEME.text)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Other",
                Style::default()
                    .fg(THEME.text_accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled("  ?        ", Style::default().fg(THEME.text_accent)),
                Span::styled("Toggle this help", Style::default().fg(THEME.text)),
            ]),
            Line::from(vec![
                Span::styled("  q        ", Style::default().fg(THEME.text_accent)),
                Span::styled("Quit", Style::default().fg(THEME.text)),
            ]),
            Line::from(vec![
                Span::styled("  g        ", Style::default().fg(THEME.text_accent)),
                Span::styled("Go to top", Style::default().fg(THEME.text)),
            ]),
            Line::from(vec![
                Span::styled("  G        ", Style::default().fg(THEME.text_accent)),
                Span::styled("Go to bottom", Style::default().fg(THEME.text)),
            ]),
            Line::from(vec![
                Span::styled("  Ctrl+D   ", Style::default().fg(THEME.text_accent)),
                Span::styled("Page down", Style::default().fg(THEME.text)),
            ]),
            Line::from(vec![
                Span::styled("  Ctrl+U   ", Style::default().fg(THEME.text_accent)),
                Span::styled("Page up", Style::default().fg(THEME.text)),
            ]),
        ];

        let paragraph = Paragraph::new(help_text)
            .block(block)
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, popup_area);
    }
}
