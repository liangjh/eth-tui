use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::components::Component;
use crate::data::types::GasInfo;
use crate::events::AppEvent;
use crate::theme::THEME;
use crate::utils;

pub struct GasTracker {
    pub info: Option<GasInfo>,
    pub loading: bool,
}

impl GasTracker {
    pub fn new() -> Self {
        Self {
            info: None,
            loading: false,
        }
    }
}

impl Component for GasTracker {
    fn handle_key(&mut self, key: KeyEvent) -> Option<AppEvent> {
        match key.code {
            KeyCode::Esc | KeyCode::Backspace => Some(AppEvent::Back),
            KeyCode::Char('r') => None, // App handles refresh
            _ => None,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let outer_block = Block::default()
            .title(" Gas Tracker ")
            .borders(Borders::ALL)
            .border_style(THEME.border_focused_style());
        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        // If loading and no data yet, show loading message
        if self.loading && self.info.is_none() {
            let loading = Paragraph::new("Loading...")
                .style(THEME.muted_style())
                .alignment(Alignment::Center);
            frame.render_widget(loading, inner);
            return;
        }

        let Some(info) = &self.info else {
            let empty = Paragraph::new("No gas data available")
                .style(THEME.muted_style())
                .alignment(Alignment::Center);
            frame.render_widget(empty, inner);
            return;
        };

        // Clone the data we need so we can drop the borrow on self
        let slow = info.slow;
        let standard = info.standard;
        let fast = info.fast;
        let base_fee = info.base_fee;
        let sparkline_data: Vec<u64> = info
            .history
            .iter()
            .map(|&wei| (wei / 1_000_000_000) as u64)
            .collect();

        // Vertical layout: gas price boxes, base fee line, sparkline
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5), // Gas price boxes
                Constraint::Length(2), // Base fee line
                Constraint::Min(3),   // Sparkline
            ])
            .split(inner);

        // --- Three gas price boxes side by side ---
        let gas_columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
            ])
            .split(chunks[0]);

        // Slow
        let slow_block = Block::default()
            .title(Span::styled(" Slow ", Style::default().fg(THEME.gas_low)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(THEME.gas_low));
        let slow_text = Paragraph::new(utils::format_gwei(slow))
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(THEME.gas_low)
                    .add_modifier(Modifier::BOLD),
            )
            .block(slow_block);
        frame.render_widget(slow_text, gas_columns[0]);

        // Standard
        let standard_block = Block::default()
            .title(Span::styled(
                " Standard ",
                Style::default().fg(THEME.gas_med),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(THEME.gas_med));
        let standard_text = Paragraph::new(utils::format_gwei(standard))
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(THEME.gas_med)
                    .add_modifier(Modifier::BOLD),
            )
            .block(standard_block);
        frame.render_widget(standard_text, gas_columns[1]);

        // Fast
        let fast_block = Block::default()
            .title(Span::styled(" Fast ", Style::default().fg(THEME.gas_high)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(THEME.gas_high));
        let fast_text = Paragraph::new(utils::format_gwei(fast))
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(THEME.gas_high)
                    .add_modifier(Modifier::BOLD),
            )
            .block(fast_block);
        frame.render_widget(fast_text, gas_columns[2]);

        // --- Base Fee line ---
        let base_fee_line = Paragraph::new(Line::from(vec![
            Span::styled("Base Fee: ", THEME.muted_style()),
            Span::styled(
                utils::format_gwei(base_fee),
                Style::default()
                    .fg(THEME.text)
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(base_fee_line, chunks[1]);

        // --- Base Fee History sparkline ---
        let sparkline_block = Block::default()
            .title(" Base Fee History ")
            .borders(Borders::ALL)
            .border_style(THEME.border_style());

        let sparkline = Sparkline::default()
            .block(sparkline_block)
            .data(&sparkline_data)
            .style(THEME.accent_style());
        frame.render_widget(sparkline, chunks[2]);
    }
}
