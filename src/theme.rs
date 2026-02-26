use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub bg: Color,
    pub surface: Color,
    pub surface_bright: Color,
    pub text: Color,
    pub text_muted: Color,
    pub text_accent: Color,
    pub success: Color,
    pub error: Color,
    pub warning: Color,
    pub info: Color,
    pub selected_bg: Color,
    pub selected_fg: Color,
    pub border: Color,
    pub border_focused: Color,
    pub eth_value: Color,
    pub address_color: Color,
    pub hash_color: Color,
    pub gas_low: Color,
    pub gas_med: Color,
    pub gas_high: Color,
}

pub const THEME: Theme = Theme {
    bg: Color::Rgb(16, 16, 28),
    surface: Color::Rgb(24, 24, 40),
    surface_bright: Color::Rgb(36, 36, 56),
    text: Color::Rgb(220, 220, 230),
    text_muted: Color::Rgb(120, 120, 140),
    text_accent: Color::Cyan,
    success: Color::Green,
    error: Color::Red,
    warning: Color::Yellow,
    info: Color::Cyan,
    selected_bg: Color::Rgb(40, 60, 100),
    selected_fg: Color::White,
    border: Color::Rgb(60, 60, 80),
    border_focused: Color::Cyan,
    eth_value: Color::Rgb(98, 126, 234),
    address_color: Color::Rgb(255, 179, 71),
    hash_color: Color::Rgb(150, 150, 180),
    gas_low: Color::Green,
    gas_med: Color::Yellow,
    gas_high: Color::Red,
};

impl Theme {
    pub const fn header_style(&self) -> Style {
        Style::new().fg(self.text).bg(self.surface)
    }

    pub const fn selected_style(&self) -> Style {
        Style::new().fg(self.selected_fg).bg(self.selected_bg).add_modifier(Modifier::BOLD)
    }

    pub const fn border_style(&self) -> Style {
        Style::new().fg(self.border)
    }

    pub const fn border_focused_style(&self) -> Style {
        Style::new().fg(self.border_focused)
    }

    pub const fn muted_style(&self) -> Style {
        Style::new().fg(self.text_muted)
    }

    pub const fn accent_style(&self) -> Style {
        Style::new().fg(self.text_accent)
    }

    pub const fn success_style(&self) -> Style {
        Style::new().fg(self.success)
    }

    pub const fn error_style(&self) -> Style {
        Style::new().fg(self.error)
    }

    pub const fn eth_style(&self) -> Style {
        Style::new().fg(self.eth_value)
    }

    pub const fn address_style(&self) -> Style {
        Style::new().fg(self.address_color)
    }

    pub const fn hash_style(&self) -> Style {
        Style::new().fg(self.hash_color)
    }

    pub fn gas_style(&self, utilization_pct: f64) -> Style {
        let color = if utilization_pct < 50.0 {
            self.gas_low
        } else if utilization_pct < 80.0 {
            self.gas_med
        } else {
            self.gas_high
        };
        Style::new().fg(color)
    }

    pub const fn table_header_style(&self) -> Style {
        Style::new().fg(self.text).bg(self.surface_bright).add_modifier(Modifier::BOLD)
    }
}
