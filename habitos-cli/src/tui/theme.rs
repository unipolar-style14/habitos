use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeKind {
    /// Uses the terminal's own ANSI palette — adapts to whatever iTerm /
    /// Terminal / Alacritty theme you have. Recommended default.
    System,
    /// True-color soft cyan/green/amber palette (the original launch theme).
    Vivid,
    /// No color at all — only bold / dim / reset. Works everywhere.
    Mono,
}

impl ThemeKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "system" | "ansi" | "default" => Some(Self::System),
            "vivid" | "rgb" => Some(Self::Vivid),
            "mono" | "none" => Some(Self::Mono),
            _ => None,
        }
    }
}

pub struct Theme {
    kind: ThemeKind,
}

impl Theme {
    pub fn new(kind: ThemeKind) -> Self {
        Self { kind }
    }

    pub fn accent(&self) -> Color {
        match self.kind {
            ThemeKind::System => Color::Cyan,
            ThemeKind::Vivid => Color::Rgb(108, 217, 255),
            ThemeKind::Mono => Color::Reset,
        }
    }
    pub fn success(&self) -> Color {
        match self.kind {
            ThemeKind::System => Color::Green,
            ThemeKind::Vivid => Color::Rgb(120, 219, 137),
            ThemeKind::Mono => Color::Reset,
        }
    }
    pub fn warn(&self) -> Color {
        match self.kind {
            ThemeKind::System => Color::Yellow,
            ThemeKind::Vivid => Color::Rgb(255, 200, 87),
            ThemeKind::Mono => Color::Reset,
        }
    }
    pub fn dim_color(&self) -> Color {
        match self.kind {
            ThemeKind::System => Color::DarkGray,
            ThemeKind::Vivid => Color::Rgb(120, 124, 142),
            ThemeKind::Mono => Color::Reset,
        }
    }
    pub fn fg_color(&self) -> Color {
        Color::Reset // let the terminal pick fg
    }

    pub fn header(&self) -> Style {
        match self.kind {
            ThemeKind::Mono => Style::default().add_modifier(Modifier::BOLD),
            _ => Style::default()
                .fg(self.accent())
                .add_modifier(Modifier::BOLD),
        }
    }
    pub fn dim(&self) -> Style {
        match self.kind {
            ThemeKind::Mono => Style::default().add_modifier(Modifier::DIM),
            _ => Style::default().fg(self.dim_color()),
        }
    }
    pub fn fg(&self) -> Style {
        match self.kind {
            ThemeKind::Mono => Style::default(),
            _ => Style::default().fg(self.fg_color()),
        }
    }
    pub fn cursor(&self) -> Style {
        match self.kind {
            ThemeKind::Mono => Style::default()
                .add_modifier(Modifier::REVERSED)
                .add_modifier(Modifier::BOLD),
            _ => Style::default()
                .fg(Color::Black)
                .bg(self.accent())
                .add_modifier(Modifier::BOLD),
        }
    }

    pub fn streak_color(&self, days: u32) -> Color {
        match self.kind {
            ThemeKind::Mono => Color::Reset,
            _ => {
                if days >= 14 {
                    Color::Magenta // hot pink in vivid, magenta in system
                } else if days >= 7 {
                    self.success()
                } else if days >= 3 {
                    self.warn()
                } else if days >= 1 {
                    Color::Reset
                } else {
                    self.dim_color()
                }
            }
        }
    }
}
