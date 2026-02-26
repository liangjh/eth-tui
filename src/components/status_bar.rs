use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::theme::THEME;
use crate::utils;

pub struct StatusBar {
    pub connected: bool,
    pub latest_block: u64,
    pub error_message: Option<String>,
    pub loading: bool,
    pub ws_connected: bool,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            connected: false,
            latest_block: 0,
            error_message: None,
            loading: false,
            ws_connected: false,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Background
        let bg = Block::default().style(THEME.header_style());
        frame.render_widget(bg, area);

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(40)])
            .split(area);

        // --- Left side ---
        let left_content = if let Some(ref err) = self.error_message {
            Line::from(vec![
                Span::styled(
                    " ! ",
                    Style::default()
                        .fg(THEME.error)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(err.as_str(), Style::default().fg(THEME.warning)),
            ])
        } else if self.loading {
            Line::from(Span::styled(
                " Loading...",
                Style::default().fg(THEME.text_accent),
            ))
        } else {
            Line::from(vec![
                Span::styled(
                    " \u{2191}\u{2193}",
                    Style::default().fg(THEME.text_accent),
                ),
                Span::styled(":Navigate  ", Style::default().fg(THEME.text_muted)),
                Span::styled("Enter", Style::default().fg(THEME.text_accent)),
                Span::styled(":Select  ", Style::default().fg(THEME.text_muted)),
                Span::styled("Esc", Style::default().fg(THEME.text_accent)),
                Span::styled(":Back  ", Style::default().fg(THEME.text_muted)),
                Span::styled("/", Style::default().fg(THEME.text_accent)),
                Span::styled(":Search  ", Style::default().fg(THEME.text_muted)),
                Span::styled("?", Style::default().fg(THEME.text_accent)),
                Span::styled(":Help  ", Style::default().fg(THEME.text_muted)),
                Span::styled("q", Style::default().fg(THEME.text_accent)),
                Span::styled(":Quit", Style::default().fg(THEME.text_muted)),
            ])
        };

        let left = Paragraph::new(left_content).style(THEME.header_style());
        frame.render_widget(left, chunks[0]);

        // --- Right side: WS status + connection status + block number ---
        let (dot_color, status_text) = if self.connected {
            (THEME.success, "Connected")
        } else {
            (THEME.error, "Disconnected")
        };

        let (ws_color, ws_text) = if self.ws_connected {
            (THEME.success, "WS")
        } else {
            (THEME.text_muted, "WS:--")
        };

        let block_str = utils::format_number(self.latest_block);

        let right_content = Line::from(vec![
            Span::styled(ws_text, Style::default().fg(ws_color)),
            Span::styled(" | ", THEME.muted_style()),
            Span::styled("\u{25cf} ", Style::default().fg(dot_color)),
            Span::styled(status_text, Style::default().fg(dot_color)),
            Span::styled(" | ", THEME.muted_style()),
            Span::styled(format!("#{block_str} "), THEME.accent_style()),
        ]);

        let right = Paragraph::new(right_content)
            .alignment(Alignment::Right)
            .style(THEME.header_style());
        frame.render_widget(right, chunks[1]);
    }
}
