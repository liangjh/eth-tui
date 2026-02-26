use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::theme::THEME;
use crate::utils;

pub struct Header {
    pub chain_id: u64,
    pub latest_block: u64,
    pub current_tab: usize,
    pub connected: bool,
    pub chain_name: String,
    pub native_symbol: String,
}

const TABS: &[&str] = &["Dashboard [1]", "Blocks [2]", "Gas [3]", "Watch [4]", "Mempool [5]"];

impl Header {
    pub fn new() -> Self {
        Self {
            chain_id: 0,
            latest_block: 0,
            current_tab: 0,
            connected: false,
            chain_name: String::new(),
            native_symbol: "ETH".to_string(),
        }
    }

    fn display_chain_name(&self) -> &str {
        if !self.chain_name.is_empty() {
            return &self.chain_name;
        }
        match self.chain_id {
            1 => "Mainnet",
            5 => "Goerli",
            11155111 => "Sepolia",
            10 => "Optimism",
            42161 => "Arbitrum",
            8453 => "Base",
            137 => "Polygon",
            _ => "Unknown",
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Background for the entire header bar
        let header_block = Block::default().style(THEME.header_style());
        frame.render_widget(header_block, area);

        // Split the header into three sections: left (title), center (tabs), right (network info)
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(12),
                Constraint::Min(0),
                Constraint::Length(28),
            ])
            .split(area);

        // Left: App title
        let title = Paragraph::new(Span::styled(
            " eth-tui",
            Style::default()
                .fg(THEME.text_accent)
                .add_modifier(Modifier::BOLD),
        ))
        .style(THEME.header_style());
        frame.render_widget(title, chunks[0]);

        // Center: Tab navigation
        let tab_titles: Vec<Line> = TABS.iter().map(|t| Line::from(*t)).collect();
        let tabs = Tabs::new(tab_titles)
            .select(self.current_tab)
            .style(THEME.muted_style())
            .highlight_style(THEME.accent_style().add_modifier(Modifier::BOLD))
            .divider(Span::raw(" | "));
        frame.render_widget(tabs, chunks[1]);

        // Right: Network info and block number
        let block_str = utils::format_number(self.latest_block);
        let network_info = Line::from(vec![
            Span::styled(self.display_chain_name(), Style::default().fg(THEME.text)),
            Span::styled(" | ", THEME.muted_style()),
            Span::styled(format!("#{block_str}"), THEME.accent_style()),
        ]);
        let network_paragraph = Paragraph::new(network_info)
            .alignment(Alignment::Right)
            .style(THEME.header_style());
        frame.render_widget(network_paragraph, chunks[2]);
    }
}
