use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::components::Component;
use crate::data::types::{DecodedLog, InternalCall, TransactionDetail, TxStatus};
use crate::events::AppEvent;
use crate::theme::THEME;
use crate::utils;

pub struct TxDetailView {
    pub detail: Option<TransactionDetail>,
    pub internal_calls: Vec<InternalCall>,
    pub decoded_logs: Vec<DecodedLog>,
    pub loading: bool,
    scroll: u16,
    max_scroll: u16,
}

impl TxDetailView {
    pub fn new() -> Self {
        Self {
            detail: None,
            internal_calls: Vec::new(),
            decoded_logs: Vec::new(),
            loading: false,
            scroll: 0,
            max_scroll: 0,
        }
    }

    fn build_lines(&self, detail: &TransactionDetail) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = Vec::new();
        let tx = &detail.summary;

        // ---- Section 1: Core Info ----
        lines.push(Line::from(vec![
            Span::styled("  Transaction Detail  ", Style::default().fg(THEME.text).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("  Hash:  ", THEME.muted_style()),
            Span::styled(format!("{}", tx.hash), THEME.hash_style()),
        ]));
        lines.push(Line::from(""));

        // Status
        let status_span = match tx.status {
            TxStatus::Success => Span::styled(
                "  Status:  ".to_string(),
                THEME.muted_style(),
            ),
            TxStatus::Failed => Span::styled(
                "  Status:  ".to_string(),
                THEME.muted_style(),
            ),
            TxStatus::Pending => Span::styled(
                "  Status:  ".to_string(),
                THEME.muted_style(),
            ),
        };
        let status_value = match tx.status {
            TxStatus::Success => Span::styled(
                "\u{2713} Success",
                Style::default().fg(THEME.success).add_modifier(Modifier::BOLD),
            ),
            TxStatus::Failed => Span::styled(
                "\u{2717} Failed",
                Style::default().fg(THEME.error).add_modifier(Modifier::BOLD),
            ),
            TxStatus::Pending => Span::styled(
                "\u{23f3} Pending",
                Style::default().fg(THEME.warning).add_modifier(Modifier::BOLD),
            ),
        };
        lines.push(Line::from(vec![status_span, status_value]));

        if let Some(block_num) = tx.block_number {
            lines.push(Line::from(vec![
                Span::styled("  Block:  ", THEME.muted_style()),
                Span::styled(format!("{block_num}"), THEME.accent_style()),
                Span::raw("    "),
                Span::styled("Confirmations:  ", THEME.muted_style()),
                Span::styled(
                    utils::format_number(detail.confirmations),
                    Style::default().fg(THEME.text),
                ),
            ]));
        }

        if tx.timestamp > 0 {
            lines.push(Line::from(vec![
                Span::styled("  Timestamp:  ", THEME.muted_style()),
                Span::raw(utils::format_timestamp(tx.timestamp)),
                Span::raw("  ("),
                Span::raw(utils::format_time_ago(tx.timestamp)),
                Span::raw(")"),
            ]));
        }

        // ---- Section 2: Parties ----
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Parties",
            Style::default().fg(THEME.text).add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )));
        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("  From:  ", THEME.muted_style()),
            Span::styled(format!("{}", tx.from), Style::default().fg(THEME.address_color)),
        ]));

        match &tx.to {
            Some(addr) => {
                lines.push(Line::from(vec![
                    Span::styled("  To:    ", THEME.muted_style()),
                    Span::styled(format!("{addr}"), Style::default().fg(THEME.address_color)),
                ]));
            }
            None => {
                lines.push(Line::from(vec![
                    Span::styled("  To:    ", THEME.muted_style()),
                    Span::styled("Contract Creation", Style::default().fg(THEME.warning)),
                ]));
            }
        }

        lines.push(Line::from(vec![
            Span::styled("  Value:  ", THEME.muted_style()),
            Span::styled(
                utils::format_eth(tx.value),
                Style::default().fg(THEME.eth_value),
            ),
        ]));

        // ---- Section 3: Gas ----
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Gas",
            Style::default().fg(THEME.text).add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )));
        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("  Gas Limit:  ", THEME.muted_style()),
            Span::raw(utils::format_number(detail.gas_limit)),
        ]));

        if let Some(gas_used) = tx.gas_used {
            let gas_pct = utils::gas_utilization_pct(gas_used, detail.gas_limit);
            lines.push(Line::from(vec![
                Span::styled("  Gas Used:   ", THEME.muted_style()),
                Span::styled(
                    format!("{} ({:.1}%)", utils::format_number(gas_used), gas_pct),
                    THEME.gas_style(gas_pct),
                ),
            ]));
        }

        if let Some(gas_price) = tx.gas_price {
            lines.push(Line::from(vec![
                Span::styled("  Gas Price:  ", THEME.muted_style()),
                Span::raw(utils::format_gwei(gas_price)),
            ]));
        }

        if let Some(max_fee) = detail.max_fee_per_gas {
            lines.push(Line::from(vec![
                Span::styled("  Max Fee:    ", THEME.muted_style()),
                Span::raw(utils::format_gwei(max_fee)),
            ]));
        }

        if let Some(priority_fee) = detail.max_priority_fee_per_gas {
            lines.push(Line::from(vec![
                Span::styled("  Priority Fee:  ", THEME.muted_style()),
                Span::raw(utils::format_gwei(priority_fee)),
            ]));
        }

        // Transaction fee = gas_used * effective_gas_price
        if let (Some(gas_used), Some(effective_price)) =
            (tx.gas_used, detail.effective_gas_price)
        {
            let fee_wei = alloy::primitives::U256::from(gas_used)
                * alloy::primitives::U256::from(effective_price);
            lines.push(Line::from(vec![
                Span::styled("  Tx Fee:  ", THEME.muted_style()),
                Span::styled(utils::format_eth(fee_wei), THEME.eth_style()),
            ]));
        }

        lines.push(Line::from(vec![
            Span::styled("  Type:  ", THEME.muted_style()),
            Span::raw(format!("{}", tx.tx_type)),
        ]));

        // ---- Section 4: Method / Decoded Input ----
        if let Some(decoded) = &detail.decoded_input {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Method",
                Style::default().fg(THEME.text).add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
            lines.push(Line::from(""));

            lines.push(Line::from(vec![
                Span::styled("  Function:  ", THEME.muted_style()),
                Span::styled(
                    decoded.function_name.clone(),
                    THEME.accent_style(),
                ),
            ]));

            if !decoded.params.is_empty() {
                lines.push(Line::from(Span::styled("  Parameters:", THEME.muted_style())));
                for (name, value) in &decoded.params {
                    lines.push(Line::from(vec![
                        Span::raw("      "),
                        Span::styled(format!("{name}: "), THEME.muted_style()),
                        Span::raw(value.clone()),
                    ]));
                }
            }
        }

        // ---- Section 5: Token Transfers ----
        if !detail.token_transfers.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Token Transfers",
                Style::default().fg(THEME.text).add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
            lines.push(Line::from(""));

            for transfer in &detail.token_transfers {
                let symbol = transfer
                    .token_symbol
                    .as_deref()
                    .unwrap_or("TOKEN");

                let decimals = transfer.decimals.unwrap_or(18);
                let amount = utils::format_u256_as_decimal(transfer.value, decimals);

                lines.push(Line::from(vec![
                    Span::styled(format!("  {symbol} "), THEME.accent_style()),
                    Span::styled(
                        utils::truncate_address(&transfer.from),
                        THEME.address_style(),
                    ),
                    Span::raw(" \u{2192} "),
                    Span::styled(
                        utils::truncate_address(&transfer.to),
                        THEME.address_style(),
                    ),
                    Span::raw(format!("  {amount}")),
                ]));
            }
        }

        // ---- Section 6: Internal Transactions ----
        if !self.internal_calls.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Internal Transactions",
                Style::default().fg(THEME.text).add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
            lines.push(Line::from(""));

            for call in &self.internal_calls {
                let indent = "  ".repeat(call.depth + 1);
                let value_str = if call.value.is_zero() {
                    String::new()
                } else {
                    format!("  {}", utils::format_eth(call.value))
                };
                let error_str = if let Some(ref err) = call.error {
                    format!(" [ERR: {err}]")
                } else {
                    String::new()
                };
                lines.push(Line::from(vec![
                    Span::raw(format!("{indent}")),
                    Span::styled(
                        call.call_type.clone(),
                        Style::default().fg(THEME.warning).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        utils::truncate_address(&call.from),
                        THEME.address_style(),
                    ),
                    Span::raw(" \u{2192} "),
                    Span::styled(
                        utils::truncate_address(&call.to),
                        THEME.address_style(),
                    ),
                    Span::styled(value_str, THEME.eth_style()),
                    Span::styled(error_str, THEME.error_style()),
                ]));
            }
        }

        // ---- Section 7: Events (Decoded Logs) ----
        if !self.decoded_logs.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Events",
                Style::default().fg(THEME.text).add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
            lines.push(Line::from(""));

            for log in &self.decoded_logs {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {} ", log.event_name),
                        THEME.accent_style(),
                    ),
                    Span::styled(
                        utils::truncate_address(&log.address),
                        THEME.address_style(),
                    ),
                ]));

                for (name, value) in &log.params {
                    lines.push(Line::from(vec![
                        Span::raw("      "),
                        Span::styled(format!("{name}: "), THEME.muted_style()),
                        Span::raw(value.clone()),
                    ]));
                }
            }
        }

        // ---- Section 8: Raw Input ----
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Raw Input",
            Style::default().fg(THEME.text).add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )));
        lines.push(Line::from(""));

        let input_hex = format!("{}", detail.input_data);
        let truncated = if input_hex.len() > 200 {
            format!("  {}...", &input_hex[..200])
        } else if input_hex.is_empty() {
            "  0x".to_string()
        } else {
            format!("  {input_hex}")
        };
        lines.push(Line::from(Span::styled(truncated, THEME.muted_style())));

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  Nonce:  ", THEME.muted_style()),
            Span::raw(format!("{}", detail.nonce)),
            Span::raw("    "),
            Span::styled("Logs:  ", THEME.muted_style()),
            Span::raw(format!("{}", detail.logs_count)),
        ]));

        lines.push(Line::from(""));

        lines
    }
}

impl Component for TxDetailView {
    fn handle_key(&mut self, key: KeyEvent) -> Option<AppEvent> {
        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) | (KeyCode::Backspace, _) => Some(AppEvent::Back),
            (KeyCode::Char('j'), _) | (KeyCode::Down, _) => {
                if self.scroll < self.max_scroll {
                    self.scroll += 1;
                }
                None
            }
            (KeyCode::Char('k'), _) | (KeyCode::Up, _) => {
                self.scroll = self.scroll.saturating_sub(1);
                None
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                // Page down
                self.scroll = self.scroll.saturating_add(20).min(self.max_scroll);
                None
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                // Page up
                self.scroll = self.scroll.saturating_sub(20);
                None
            }
            (KeyCode::Char('g'), _) => {
                self.scroll = 0;
                None
            }
            (KeyCode::Char('G'), _) => {
                self.scroll = self.max_scroll;
                None
            }
            _ => None,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let outer_block = Block::default()
            .title(" Transaction Detail ")
            .borders(Borders::ALL)
            .border_style(THEME.border_focused_style());

        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        // Show loading state
        if self.loading && self.detail.is_none() {
            let loading = Paragraph::new("Loading...")
                .style(THEME.muted_style())
                .alignment(Alignment::Center);
            frame.render_widget(loading, inner);
            return;
        }

        let detail = match &self.detail {
            Some(d) => d,
            None => return,
        };

        let lines = self.build_lines(detail);
        let total_lines = lines.len() as u16;
        self.max_scroll = total_lines.saturating_sub(inner.height);

        // Clamp scroll
        if self.scroll > self.max_scroll {
            self.scroll = self.max_scroll;
        }

        let paragraph = Paragraph::new(lines)
            .style(Style::default().fg(THEME.text))
            .scroll((self.scroll, 0));

        frame.render_widget(paragraph, inner);

        // Render scrollbar if content exceeds area
        if total_lines > inner.height {
            let mut scroll_state = ScrollbarState::default()
                .content_length(total_lines as usize)
                .position(self.scroll as usize);

            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("^"))
                .end_symbol(Some("v"));

            frame.render_stateful_widget(scrollbar, inner, &mut scroll_state);
        }
    }
}
