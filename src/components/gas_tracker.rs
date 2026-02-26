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
        let blob_base_fee = info.blob_base_fee;
        let is_congested = info.is_congested;
        let priority_fee_percentiles = info.priority_fee_percentiles.clone();
        let sparkline_data: Vec<u64> = info
            .history
            .iter()
            .map(|&wei| (wei / 1_000_000_000) as u64)
            .collect();

        // Determine layout constraints based on available data
        let has_percentiles = !priority_fee_percentiles.is_empty();
        let constraints = if has_percentiles {
            vec![
                Constraint::Length(5), // Gas price boxes
                Constraint::Length(3), // Base fee + blob fee + congestion
                Constraint::Length(5), // Priority fee percentiles
                Constraint::Min(3),   // Sparkline
            ]
        } else {
            vec![
                Constraint::Length(5), // Gas price boxes
                Constraint::Length(3), // Base fee + blob fee + congestion
                Constraint::Min(3),   // Sparkline
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
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

        // --- Base Fee, Blob Fee, and Congestion lines ---
        let mut info_lines: Vec<Line<'static>> = Vec::new();

        // Base fee line
        let mut base_spans = vec![
            Span::styled("Base Fee: ", THEME.muted_style()),
            Span::styled(
                utils::format_gwei(base_fee),
                Style::default()
                    .fg(THEME.text)
                    .add_modifier(Modifier::BOLD),
            ),
        ];

        // Append blob base fee on same line if available
        if let Some(blob_fee) = blob_base_fee {
            base_spans.push(Span::raw("    "));
            base_spans.push(Span::styled("Blob Fee: ", THEME.muted_style()));
            base_spans.push(Span::styled(
                utils::format_gwei(blob_fee),
                Style::default()
                    .fg(THEME.text)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        info_lines.push(Line::from(base_spans));

        // Network congestion indicator
        let (congestion_label, congestion_color) = if is_congested {
            ("Congested", THEME.gas_high)
        } else {
            ("Normal", THEME.gas_low)
        };
        info_lines.push(Line::from(vec![
            Span::styled("Network: ", THEME.muted_style()),
            Span::styled(
                congestion_label,
                Style::default().fg(congestion_color).add_modifier(Modifier::BOLD),
            ),
        ]));

        let info_paragraph = Paragraph::new(info_lines).alignment(Alignment::Center);
        frame.render_widget(info_paragraph, chunks[1]);

        // --- Priority fee percentile bars (if available) ---
        let sparkline_chunk_idx;
        if has_percentiles {
            sparkline_chunk_idx = 3;

            let percentile_block = Block::default()
                .title(" Priority Fee Percentiles ")
                .borders(Borders::ALL)
                .border_style(THEME.border_style());
            let percentile_inner = percentile_block.inner(chunks[2]);
            frame.render_widget(percentile_block, chunks[2]);

            // Build bar chart data from percentiles
            let mut bar_labels: Vec<String> = Vec::new();
            let mut bar_values: Vec<u64> = Vec::new();
            for (pct, fee) in &priority_fee_percentiles {
                bar_labels.push(format!("p{pct}"));
                // Convert to gwei for display
                bar_values.push((*fee / 1_000_000_000) as u64);
            }

            // Render as text-based bars since BarChart requires specific data format
            let max_val = bar_values.iter().copied().max().unwrap_or(1).max(1);
            let mut percentile_lines: Vec<Line<'static>> = Vec::new();
            for (i, label) in bar_labels.iter().enumerate() {
                let val = bar_values[i];
                let bar_color = if i < 2 {
                    THEME.gas_low
                } else if i < 4 {
                    THEME.gas_med
                } else {
                    THEME.gas_high
                };
                let bar_width = if percentile_inner.width > 20 {
                    ((val as f64 / max_val as f64) * (percentile_inner.width as f64 - 20.0)) as usize
                } else {
                    0
                };
                let bar_str: String = "\u{2588}".repeat(bar_width);
                let fee_gwei = priority_fee_percentiles[i].1 as f64 / 1e9;
                percentile_lines.push(Line::from(vec![
                    Span::styled(format!("{label:>4} "), THEME.muted_style()),
                    Span::styled(bar_str, Style::default().fg(bar_color)),
                    Span::raw(format!(" {fee_gwei:.2} Gwei")),
                ]));
            }

            let percentile_paragraph = Paragraph::new(percentile_lines);
            frame.render_widget(percentile_paragraph, percentile_inner);
        } else {
            sparkline_chunk_idx = 2;
        }

        // --- Base Fee History sparkline ---
        let sparkline_block = Block::default()
            .title(" Base Fee History ")
            .borders(Borders::ALL)
            .border_style(THEME.border_style());

        let sparkline = Sparkline::default()
            .block(sparkline_block)
            .data(&sparkline_data)
            .style(THEME.accent_style());
        frame.render_widget(sparkline, chunks[sparkline_chunk_idx]);
    }
}
