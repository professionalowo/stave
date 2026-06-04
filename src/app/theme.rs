use ratatui::style::{Color, Modifier, Style};
use serde::Deserialize;
use std::fs;
use std::str::FromStr;

#[derive(Debug, Deserialize)]
struct ThemeConfig {
    header: Option<String>,
    selection_bg: Option<String>,
    selection_fg: Option<String>,
    sidebar_active: Option<String>,
    sidebar_inactive: Option<String>,
    accent: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct Palette {
    header: Color,
    selection_bg: Color,
    selection_fg: Color,
    sidebar_active: Color,
    sidebar_inactive: Color,
    accent: Color,
}

impl Default for Palette {
    fn default() -> Self {
        // Default to a fallback palette
        Self {
            header: Color::Rgb(242, 213, 207),
            accent: Color::Rgb(202, 158, 230),
            sidebar_active: Color::Rgb(252, 211, 77),
            selection_bg: Color::Rgb(129, 200, 190),
            selection_fg: Color::Rgb(48, 52, 70),
            sidebar_inactive: Color::Rgb(165, 173, 206),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    palette: Palette,
    colors_enabled: bool,
}

impl Theme {
    pub fn load(colors_enabled: bool) -> Self {
        let mut palette = Palette::default();

        let mut config_paths = Vec::new();

        // 1. Standard config dir (e.g. ~/Library/Application Support/stave on macOS, ~/.config/stave on Linux)
        if let Some(mut path) = dirs::config_dir() {
            path.push("stave");
            path.push("theme.yml");
            config_paths.push(path);
        }

        // 2. Fallback to ~/.config/stave on macOS/Windows just in case users put it there (XDG preference)
        if let Some(mut path) = dirs::home_dir() {
            path.push(".config");
            path.push("stave");
            path.push("theme.yml");
            config_paths.push(path);
        }

        let mut loaded = false;
        for path in config_paths {
            if !loaded {
                if let Ok(file) = fs::File::open(&path) {
                    if let Ok(config) = serde_yaml::from_reader::<_, ThemeConfig>(file) {
                        if let Some(c) = config.header { if let Ok(color) = Color::from_str(&c) { palette.header = color; } }
                        if let Some(c) = config.selection_bg { if let Ok(color) = Color::from_str(&c) { palette.selection_bg = color; } }
                        if let Some(c) = config.selection_fg { if let Ok(color) = Color::from_str(&c) { palette.selection_fg = color; } }
                        if let Some(c) = config.sidebar_active { if let Ok(color) = Color::from_str(&c) { palette.sidebar_active = color; } }
                        if let Some(c) = config.sidebar_inactive { if let Ok(color) = Color::from_str(&c) { palette.sidebar_inactive = color; } }
                        if let Some(c) = config.accent { if let Ok(color) = Color::from_str(&c) { palette.accent = color; } }
                        loaded = true;
                    }
                }
            }
        }

        Self {
            palette,
            colors_enabled,
        }
    }

    pub fn header_style(self) -> Style {
        if !self.colors_enabled {
            return Style::default().add_modifier(Modifier::BOLD);
        }

        Style::default()
            .fg(self.palette.header)
            .add_modifier(Modifier::BOLD)
    }

    pub fn selection_style(self) -> Style {
        if !self.colors_enabled {
            return Style::default().add_modifier(Modifier::REVERSED);
        }

        Style::default()
            .fg(self.palette.selection_fg)
            .bg(self.palette.selection_bg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn sidebar_style(self, active: bool) -> Style {
        if !self.colors_enabled {
            return Style::default();
        }

        if active {
            Style::default().fg(self.palette.sidebar_active).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.palette.sidebar_inactive)
        }
    }

    pub fn accent_style(self) -> Style {
        if !self.colors_enabled {
            return Style::default();
        }

        Style::default().fg(self.palette.accent)
    }
}
