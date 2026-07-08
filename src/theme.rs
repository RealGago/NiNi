use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::path::Path;

const THEME_PATH: &str = "theme.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub primary: (u8, u8, u8),
    pub accent: (u8, u8, u8),
    pub code: (u8, u8, u8),
    pub code_bg: (u8, u8, u8),
    pub error: (u8, u8, u8),
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            primary: (147, 112, 219), // Purple Now
            accent: (100, 200, 255),
            code: (180, 220, 180),
            code_bg: (30, 30, 40),
            error: (255, 80, 80),
        }
    }
}

impl Theme {
    pub fn primary(&self) -> Color {
        let (r, g, b) = self.primary;
        Color::Rgb(r, g, b)
    }
    pub fn accent(&self) -> Color {
        let (r, g, b) = self.accent;
        Color::Rgb(r, g, b)
    }
    pub fn code(&self) -> Color {
        let (r, g, b) = self.code;
        Color::Rgb(r, g, b)
    }
    pub fn code_bg(&self) -> Color {
        let (r, g, b) = self.code_bg;
        Color::Rgb(r, g, b)
    }
    pub fn error(&self) -> Color {
        let (r, g, b) = self.error;
        Color::Rgb(r, g, b)
    }

    // Read theme.toml in project source. If dont exist or parse error, 
    // fall to default
    pub fn load() -> Self {
        if Path::new(THEME_PATH).exists() {
            match std::fs::read_to_string(THEME_PATH) {
                Ok(raw) => toml::from_str(&raw).unwrap_or_default(),
                Err(_) => Theme::default(),
            }
        } else {
            Theme::default()
        }
    }

    // Save actual state in theme.toml. Call every update in the colors
    pub fn save(&self) -> anyhow::Result<()> {
        let raw = toml::to_string_pretty(self)?;
        std::fs::write(THEME_PATH, raw)?;
        Ok(())
    }

    pub fn get(&self, field: crate::app::ThemeField) -> (u8, u8, u8) {
        use crate::app::ThemeField::*;
        match field {
            Primary => self.primary,
            Accent => self.accent,
            Code => self.code,
            CodeBg => self.code_bg,
            Error => self.error,
        }
    }

    pub fn set_channel(&mut self, field: crate::app::ThemeField, channel: crate::app::RgbChannel, value: u8) {
        use crate::app::ThemeField::*;
        use crate::app::RgbChannel::*;
        let target = match field {
            Primary => &mut self.primary,
            Accent => &mut self.accent,
            Code => &mut self.code,
            CodeBg => &mut self.code_bg,
            Error => &mut self.error,
        };
        match channel {
            R => target.0 = value,
            G => target.1 = value,
            B => target.2 = value,
        }
    }
}
