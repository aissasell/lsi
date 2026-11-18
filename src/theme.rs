use anyhow::{bail, Context, Result};
use clap::ValueEnum;
use ratatui::style::Color;
use serde::Deserialize;

/// Built-in and custom color themes supported by `lsi`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeName {
    Custom,
    #[default]
    EzaDark,
    Catppuccin,
    Dracula,
    Nord,
    Gruvbox,
    TokyoNight,
    SolarizedLight,
}

impl ThemeName {
    /// All built-in themes in cycling order.
    #[allow(dead_code)]
    pub const BUILTIN: [Self; 7] = [
        Self::EzaDark,
        Self::Catppuccin,
        Self::Dracula,
        Self::Nord,
        Self::Gruvbox,
        Self::TokyoNight,
        Self::SolarizedLight,
    ];

    /// Human-readable kebab-case display name matching CLI `--theme` values.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Custom => "custom",
            Self::EzaDark => "eza-dark",
            Self::Catppuccin => "catppuccin",
            Self::Dracula => "dracula",
            Self::Nord => "nord",
            Self::Gruvbox => "gruvbox",
            Self::TokyoNight => "tokyo-night",
            Self::SolarizedLight => "solarized-light",
        }
    }

    /// Returns the next theme in the cycle.
    pub const fn next(self, has_custom: bool) -> Self {
        match self {
            Self::Custom => Self::EzaDark,
            Self::EzaDark => Self::Catppuccin,
            Self::Catppuccin => Self::Dracula,
            Self::Dracula => Self::Nord,
            Self::Nord => Self::Gruvbox,
            Self::Gruvbox => Self::TokyoNight,
            Self::TokyoNight => Self::SolarizedLight,
            Self::SolarizedLight => {
                if has_custom {
                    Self::Custom
                } else {
                    Self::EzaDark
                }
            }
        }
    }

    /// Returns the previous theme in the cycle.
    #[allow(dead_code)]
    pub const fn prev(self, has_custom: bool) -> Self {
        match self {
            Self::Custom => Self::SolarizedLight,
            Self::EzaDark => {
                if has_custom {
                    Self::Custom
                } else {
                    Self::SolarizedLight
                }
            }
            Self::Catppuccin => Self::EzaDark,
            Self::Dracula => Self::Catppuccin,
            Self::Nord => Self::Dracula,
            Self::Gruvbox => Self::Nord,
            Self::TokyoNight => Self::Gruvbox,
            Self::SolarizedLight => Self::TokyoNight,
        }
    }

    /// Builds the concrete color palette for this theme.
    pub const fn palette(self) -> Theme {
        match self {
            Self::Custom | Self::EzaDark => Theme {
                name: self,
                bg: Color::Rgb(16, 20, 26),
                fg: Color::Rgb(220, 224, 232),
                footer_bg: Color::Rgb(22, 27, 36),
                footer_fg: Color::Rgb(180, 190, 205),
                border: Color::Rgb(62, 74, 92),
                selected_bg: Color::Rgb(42, 56, 78),
                selected_fg: Color::Rgb(255, 255, 255),
                accent: Color::Rgb(97, 175, 239),
                muted: Color::Rgb(115, 125, 145),
                dir_fg: Color::Rgb(97, 175, 239),
                symlink_fg: Color::Rgb(86, 182, 194),
                broken_symlink_fg: Color::Rgb(224, 108, 117),
                exec_fg: Color::Rgb(152, 195, 121),
                hidden_fg: Color::Rgb(128, 138, 156),
                file_fg: Color::Rgb(220, 224, 232),
                code_fg: Color::Rgb(229, 192, 123),
                config_fg: Color::Rgb(198, 120, 221),
                doc_fg: Color::Rgb(97, 175, 239),
                archive_fg: Color::Rgb(224, 108, 117),
                media_fg: Color::Rgb(198, 120, 221),
                perm_dir: Color::Rgb(97, 175, 239),
                perm_read: Color::Rgb(229, 192, 123),
                perm_write: Color::Rgb(224, 108, 117),
                perm_exec: Color::Rgb(152, 195, 121),
                perm_special: Color::Rgb(198, 120, 221),
                perm_none: Color::Rgb(92, 99, 112),
                size_number: Color::Rgb(152, 195, 121),
                size_unit: Color::Rgb(105, 155, 95),
                owner_self: Color::Rgb(229, 192, 123),
                owner_other: Color::Rgb(209, 154, 102),
                group_fg: Color::Rgb(171, 178, 191),
                date_fg: Color::Rgb(86, 182, 194),
            },
            Self::Catppuccin => Theme {
                name: self,
                bg: Color::Rgb(30, 30, 46),
                fg: Color::Rgb(205, 214, 244),
                footer_bg: Color::Rgb(24, 24, 37),
                footer_fg: Color::Rgb(166, 173, 200),
                border: Color::Rgb(88, 91, 112),
                selected_bg: Color::Rgb(69, 71, 90),
                selected_fg: Color::Rgb(245, 224, 220),
                accent: Color::Rgb(137, 180, 250),
                muted: Color::Rgb(127, 132, 156),
                dir_fg: Color::Rgb(137, 180, 250),
                symlink_fg: Color::Rgb(148, 226, 213),
                broken_symlink_fg: Color::Rgb(243, 139, 168),
                exec_fg: Color::Rgb(166, 227, 161),
                hidden_fg: Color::Rgb(147, 153, 178),
                file_fg: Color::Rgb(205, 214, 244),
                code_fg: Color::Rgb(249, 226, 175),
                config_fg: Color::Rgb(203, 166, 247),
                doc_fg: Color::Rgb(116, 199, 236),
                archive_fg: Color::Rgb(243, 139, 168),
                media_fg: Color::Rgb(245, 194, 231),
                perm_dir: Color::Rgb(137, 180, 250),
                perm_read: Color::Rgb(249, 226, 175),
                perm_write: Color::Rgb(243, 139, 168),
                perm_exec: Color::Rgb(166, 227, 161),
                perm_special: Color::Rgb(203, 166, 247),
                perm_none: Color::Rgb(108, 112, 134),
                size_number: Color::Rgb(166, 227, 161),
                size_unit: Color::Rgb(148, 226, 213),
                owner_self: Color::Rgb(249, 226, 175),
                owner_other: Color::Rgb(250, 179, 135),
                group_fg: Color::Rgb(186, 194, 222),
                date_fg: Color::Rgb(137, 220, 235),
            },
            Self::Dracula => Theme {
                name: self,
                bg: Color::Rgb(40, 42, 54),
                fg: Color::Rgb(248, 248, 242),
                footer_bg: Color::Rgb(33, 34, 44),
                footer_fg: Color::Rgb(189, 147, 249),
                border: Color::Rgb(98, 114, 164),
                selected_bg: Color::Rgb(68, 71, 90),
                selected_fg: Color::Rgb(248, 248, 242),
                accent: Color::Rgb(189, 147, 249),
                muted: Color::Rgb(98, 114, 164),
                dir_fg: Color::Rgb(139, 233, 253),
                symlink_fg: Color::Rgb(255, 121, 198),
                broken_symlink_fg: Color::Rgb(255, 85, 85),
                exec_fg: Color::Rgb(80, 250, 123),
                hidden_fg: Color::Rgb(98, 114, 164),
                file_fg: Color::Rgb(248, 248, 242),
                code_fg: Color::Rgb(241, 250, 140),
                config_fg: Color::Rgb(189, 147, 249),
                doc_fg: Color::Rgb(139, 233, 253),
                archive_fg: Color::Rgb(255, 85, 85),
                media_fg: Color::Rgb(255, 121, 198),
                perm_dir: Color::Rgb(139, 233, 253),
                perm_read: Color::Rgb(241, 250, 140),
                perm_write: Color::Rgb(255, 85, 85),
                perm_exec: Color::Rgb(80, 250, 123),
                perm_special: Color::Rgb(189, 147, 249),
                perm_none: Color::Rgb(98, 114, 164),
                size_number: Color::Rgb(80, 250, 123),
                size_unit: Color::Rgb(139, 233, 253),
                owner_self: Color::Rgb(255, 184, 108),
                owner_other: Color::Rgb(255, 85, 85),
                group_fg: Color::Rgb(248, 248, 242),
                date_fg: Color::Rgb(189, 147, 249),
            },
            Self::Nord => Theme {
                name: self,
                bg: Color::Rgb(46, 52, 64),
                fg: Color::Rgb(236, 239, 244),
                footer_bg: Color::Rgb(59, 66, 82),
                footer_fg: Color::Rgb(216, 222, 233),
                border: Color::Rgb(76, 86, 106),
                selected_bg: Color::Rgb(67, 76, 94),
                selected_fg: Color::Rgb(236, 239, 244),
                accent: Color::Rgb(136, 192, 208),
                muted: Color::Rgb(129, 140, 160),
                dir_fg: Color::Rgb(129, 161, 193),
                symlink_fg: Color::Rgb(143, 188, 187),
                broken_symlink_fg: Color::Rgb(191, 97, 106),
                exec_fg: Color::Rgb(163, 190, 140),
                hidden_fg: Color::Rgb(129, 140, 160),
                file_fg: Color::Rgb(229, 233, 240),
                code_fg: Color::Rgb(235, 203, 139),
                config_fg: Color::Rgb(180, 142, 173),
                doc_fg: Color::Rgb(136, 192, 208),
                archive_fg: Color::Rgb(191, 97, 106),
                media_fg: Color::Rgb(180, 142, 173),
                perm_dir: Color::Rgb(129, 161, 193),
                perm_read: Color::Rgb(235, 203, 139),
                perm_write: Color::Rgb(191, 97, 106),
                perm_exec: Color::Rgb(163, 190, 140),
                perm_special: Color::Rgb(180, 142, 173),
                perm_none: Color::Rgb(76, 86, 106),
                size_number: Color::Rgb(163, 190, 140),
                size_unit: Color::Rgb(143, 188, 187),
                owner_self: Color::Rgb(235, 203, 139),
                owner_other: Color::Rgb(208, 135, 112),
                group_fg: Color::Rgb(216, 222, 233),
                date_fg: Color::Rgb(136, 192, 208),
            },
            Self::Gruvbox => Theme {
                name: self,
                bg: Color::Rgb(40, 40, 40),
                fg: Color::Rgb(235, 219, 178),
                footer_bg: Color::Rgb(50, 48, 47),
                footer_fg: Color::Rgb(213, 196, 161),
                border: Color::Rgb(102, 92, 84),
                selected_bg: Color::Rgb(80, 73, 69),
                selected_fg: Color::Rgb(251, 241, 199),
                accent: Color::Rgb(250, 189, 47),
                muted: Color::Rgb(146, 131, 116),
                dir_fg: Color::Rgb(131, 165, 152),
                symlink_fg: Color::Rgb(142, 192, 124),
                broken_symlink_fg: Color::Rgb(251, 73, 52),
                exec_fg: Color::Rgb(184, 187, 38),
                hidden_fg: Color::Rgb(146, 131, 116),
                file_fg: Color::Rgb(235, 219, 178),
                code_fg: Color::Rgb(250, 189, 47),
                config_fg: Color::Rgb(211, 134, 155),
                doc_fg: Color::Rgb(131, 165, 152),
                archive_fg: Color::Rgb(251, 73, 52),
                media_fg: Color::Rgb(211, 134, 155),
                perm_dir: Color::Rgb(131, 165, 152),
                perm_read: Color::Rgb(250, 189, 47),
                perm_write: Color::Rgb(251, 73, 52),
                perm_exec: Color::Rgb(184, 187, 38),
                perm_special: Color::Rgb(211, 134, 155),
                perm_none: Color::Rgb(124, 111, 100),
                size_number: Color::Rgb(184, 187, 38),
                size_unit: Color::Rgb(142, 192, 124),
                owner_self: Color::Rgb(250, 189, 47),
                owner_other: Color::Rgb(254, 128, 25),
                group_fg: Color::Rgb(213, 196, 161),
                date_fg: Color::Rgb(142, 192, 124),
            },
            Self::TokyoNight => Theme {
                name: self,
                bg: Color::Rgb(26, 27, 38),
                fg: Color::Rgb(192, 202, 245),
                footer_bg: Color::Rgb(22, 22, 30),
                footer_fg: Color::Rgb(169, 177, 214),
                border: Color::Rgb(65, 72, 104),
                selected_bg: Color::Rgb(41, 46, 66),
                selected_fg: Color::Rgb(255, 255, 255),
                accent: Color::Rgb(122, 162, 247),
                muted: Color::Rgb(86, 95, 137),
                dir_fg: Color::Rgb(122, 162, 247),
                symlink_fg: Color::Rgb(115, 218, 202),
                broken_symlink_fg: Color::Rgb(247, 118, 142),
                exec_fg: Color::Rgb(158, 206, 106),
                hidden_fg: Color::Rgb(86, 95, 137),
                file_fg: Color::Rgb(192, 202, 245),
                code_fg: Color::Rgb(224, 175, 104),
                config_fg: Color::Rgb(187, 154, 247),
                doc_fg: Color::Rgb(125, 207, 255),
                archive_fg: Color::Rgb(247, 118, 142),
                media_fg: Color::Rgb(187, 154, 247),
                perm_dir: Color::Rgb(122, 162, 247),
                perm_read: Color::Rgb(224, 175, 104),
                perm_write: Color::Rgb(247, 118, 142),
                perm_exec: Color::Rgb(158, 206, 106),
                perm_special: Color::Rgb(187, 154, 247),
                perm_none: Color::Rgb(65, 72, 104),
                size_number: Color::Rgb(158, 206, 106),
                size_unit: Color::Rgb(115, 218, 202),
                owner_self: Color::Rgb(224, 175, 104),
                owner_other: Color::Rgb(255, 158, 100),
                group_fg: Color::Rgb(169, 177, 214),
                date_fg: Color::Rgb(125, 207, 255),
            },
            Self::SolarizedLight => Theme {
                name: self,
                bg: Color::Rgb(253, 246, 227),
                fg: Color::Rgb(88, 110, 117),
                footer_bg: Color::Rgb(238, 232, 213),
                footer_fg: Color::Rgb(88, 110, 117),
                border: Color::Rgb(147, 161, 161),
                selected_bg: Color::Rgb(211, 203, 184),
                selected_fg: Color::Rgb(7, 54, 66),
                accent: Color::Rgb(38, 139, 210),
                muted: Color::Rgb(147, 161, 161),
                dir_fg: Color::Rgb(38, 139, 210),
                symlink_fg: Color::Rgb(42, 161, 152),
                broken_symlink_fg: Color::Rgb(220, 50, 47),
                exec_fg: Color::Rgb(133, 153, 0),
                hidden_fg: Color::Rgb(147, 161, 161),
                file_fg: Color::Rgb(88, 110, 117),
                code_fg: Color::Rgb(181, 137, 0),
                config_fg: Color::Rgb(108, 113, 196),
                doc_fg: Color::Rgb(38, 139, 210),
                archive_fg: Color::Rgb(220, 50, 47),
                media_fg: Color::Rgb(211, 54, 130),
                perm_dir: Color::Rgb(38, 139, 210),
                perm_read: Color::Rgb(181, 137, 0),
                perm_write: Color::Rgb(220, 50, 47),
                perm_exec: Color::Rgb(133, 153, 0),
                perm_special: Color::Rgb(108, 113, 196),
                perm_none: Color::Rgb(147, 161, 161),
                size_number: Color::Rgb(133, 153, 0),
                size_unit: Color::Rgb(42, 161, 152),
                owner_self: Color::Rgb(181, 137, 0),
                owner_other: Color::Rgb(203, 75, 22),
                group_fg: Color::Rgb(101, 123, 131),
                date_fg: Color::Rgb(42, 161, 152),
            },
        }
    }
}

/// Concrete semantic color palette for rendering the TUI and eza-style columns.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct Theme {
    #[allow(dead_code)]
    pub name: ThemeName,
    pub bg: Color,
    pub fg: Color,
    pub footer_bg: Color,
    pub footer_fg: Color,
    pub border: Color,
    pub selected_bg: Color,
    pub selected_fg: Color,
    pub accent: Color,
    pub muted: Color,

    pub dir_fg: Color,
    pub symlink_fg: Color,
    pub broken_symlink_fg: Color,
    pub exec_fg: Color,
    pub hidden_fg: Color,
    pub file_fg: Color,

    pub code_fg: Color,
    pub config_fg: Color,
    pub doc_fg: Color,
    pub archive_fg: Color,
    pub media_fg: Color,

    pub perm_dir: Color,
    pub perm_read: Color,
    pub perm_write: Color,
    pub perm_exec: Color,
    pub perm_special: Color,
    pub perm_none: Color,

    pub size_number: Color,
    pub size_unit: Color,

    pub owner_self: Color,
    pub owner_other: Color,
    pub group_fg: Color,

    pub date_fg: Color,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum ColorSpec {
    Str(String),
    Rgb([u8; 3]),
}

impl ColorSpec {
    fn to_color(&self) -> Result<Color> {
        match self {
            Self::Rgb([r, g, b]) => Ok(Color::Rgb(*r, *g, *b)),
            Self::Str(raw) => parse_color_str(raw),
        }
    }
}

fn parse_color_str(raw: &str) -> Result<Color> {
    let trimmed = raw.trim();
    if let Some(hex) = trimmed.strip_prefix('#') {
        return parse_hex_rgb(hex);
    }

    let lower = trimmed.to_ascii_lowercase();
    if let Some(inner) = lower.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() != 3 {
            bail!("Invalid rgb() color '{trimmed}'");
        }
        let r: u8 = parts[0]
            .trim()
            .parse()
            .with_context(|| format!("Invalid red component in '{trimmed}'"))?;
        let g: u8 = parts[1]
            .trim()
            .parse()
            .with_context(|| format!("Invalid green component in '{trimmed}'"))?;
        let b: u8 = parts[2]
            .trim()
            .parse()
            .with_context(|| format!("Invalid blue component in '{trimmed}'"))?;
        return Ok(Color::Rgb(r, g, b));
    }

    if lower.len() == 6 && lower.bytes().all(|b| b.is_ascii_hexdigit()) {
        return parse_hex_rgb(&lower);
    }

    let normalized = lower.replace(['-', '_', ' '], "");
    let color = match normalized.as_str() {
        "reset" | "default" => Color::Reset,
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" | "purple" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "darkgray" | "darkgrey" => Color::DarkGray,
        "lightred" => Color::LightRed,
        "lightgreen" => Color::LightGreen,
        "lightyellow" => Color::LightYellow,
        "lightblue" => Color::LightBlue,
        "lightmagenta" | "lightpurple" => Color::LightMagenta,
        "lightcyan" => Color::LightCyan,
        "white" => Color::White,
        _ => bail!("Unknown color '{trimmed}'"),
    };
    Ok(color)
}

fn parse_hex_rgb(hex: &str) -> Result<Color> {
    if hex.len() != 6 {
        bail!("Expected 6-digit hex color, got '#{hex}'");
    }
    let r = u8::from_str_radix(&hex[0..2], 16)
        .with_context(|| format!("Invalid hex red in '#{hex}'"))?;
    let g = u8::from_str_radix(&hex[2..4], 16)
        .with_context(|| format!("Invalid hex green in '#{hex}'"))?;
    let b = u8::from_str_radix(&hex[4..6], 16)
        .with_context(|| format!("Invalid hex blue in '#{hex}'"))?;
    Ok(Color::Rgb(r, g, b))
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ThemeConfig {
    bg: Option<ColorSpec>,
    fg: Option<ColorSpec>,
    footer_bg: Option<ColorSpec>,
    footer_fg: Option<ColorSpec>,
    border: Option<ColorSpec>,
    selected_bg: Option<ColorSpec>,
    selected_fg: Option<ColorSpec>,
    accent: Option<ColorSpec>,
    muted: Option<ColorSpec>,

    dir_fg: Option<ColorSpec>,
    symlink_fg: Option<ColorSpec>,
    broken_symlink_fg: Option<ColorSpec>,
    exec_fg: Option<ColorSpec>,
    hidden_fg: Option<ColorSpec>,
    file_fg: Option<ColorSpec>,

    code_fg: Option<ColorSpec>,
    config_fg: Option<ColorSpec>,
    doc_fg: Option<ColorSpec>,
    archive_fg: Option<ColorSpec>,
    media_fg: Option<ColorSpec>,

    perm_dir: Option<ColorSpec>,
    perm_read: Option<ColorSpec>,
    perm_write: Option<ColorSpec>,
    perm_exec: Option<ColorSpec>,
    perm_special: Option<ColorSpec>,
    perm_none: Option<ColorSpec>,

    size_number: Option<ColorSpec>,
    size_unit: Option<ColorSpec>,

    owner_self: Option<ColorSpec>,
    owner_other: Option<ColorSpec>,
    group_fg: Option<ColorSpec>,

    date_fg: Option<ColorSpec>,
}

fn resolve_opt_color(spec: Option<ColorSpec>, default: Color) -> Result<Color> {
    match spec {
        Some(c) => c.to_color(),
        None => Ok(default),
    }
}

impl Theme {
    /// Parses a custom theme from TOML content, merging omitted colors from `eza-dark`.
    pub fn from_toml_str(content: &str) -> Result<Self> {
        let cfg: ThemeConfig = toml::from_str(content).context("Failed to parse theme.toml")?;
        let base = ThemeName::EzaDark.palette();

        Ok(Self {
            name: ThemeName::Custom,
            bg: resolve_opt_color(cfg.bg, base.bg)?,
            fg: resolve_opt_color(cfg.fg, base.fg)?,
            footer_bg: resolve_opt_color(cfg.footer_bg, base.footer_bg)?,
            footer_fg: resolve_opt_color(cfg.footer_fg, base.footer_fg)?,
            border: resolve_opt_color(cfg.border, base.border)?,
            selected_bg: resolve_opt_color(cfg.selected_bg, base.selected_bg)?,
            selected_fg: resolve_opt_color(cfg.selected_fg, base.selected_fg)?,
            accent: resolve_opt_color(cfg.accent, base.accent)?,
            muted: resolve_opt_color(cfg.muted, base.muted)?,
            dir_fg: resolve_opt_color(cfg.dir_fg, base.dir_fg)?,
            symlink_fg: resolve_opt_color(cfg.symlink_fg, base.symlink_fg)?,
            broken_symlink_fg: resolve_opt_color(cfg.broken_symlink_fg, base.broken_symlink_fg)?,
            exec_fg: resolve_opt_color(cfg.exec_fg, base.exec_fg)?,
            hidden_fg: resolve_opt_color(cfg.hidden_fg, base.hidden_fg)?,
            file_fg: resolve_opt_color(cfg.file_fg, base.file_fg)?,
            code_fg: resolve_opt_color(cfg.code_fg, base.code_fg)?,
            config_fg: resolve_opt_color(cfg.config_fg, base.config_fg)?,
            doc_fg: resolve_opt_color(cfg.doc_fg, base.doc_fg)?,
            archive_fg: resolve_opt_color(cfg.archive_fg, base.archive_fg)?,
            media_fg: resolve_opt_color(cfg.media_fg, base.media_fg)?,
            perm_dir: resolve_opt_color(cfg.perm_dir, base.perm_dir)?,
            perm_read: resolve_opt_color(cfg.perm_read, base.perm_read)?,
            perm_write: resolve_opt_color(cfg.perm_write, base.perm_write)?,
            perm_exec: resolve_opt_color(cfg.perm_exec, base.perm_exec)?,
            perm_special: resolve_opt_color(cfg.perm_special, base.perm_special)?,
            perm_none: resolve_opt_color(cfg.perm_none, base.perm_none)?,
            size_number: resolve_opt_color(cfg.size_number, base.size_number)?,
            size_unit: resolve_opt_color(cfg.size_unit, base.size_unit)?,
            owner_self: resolve_opt_color(cfg.owner_self, base.owner_self)?,
            owner_other: resolve_opt_color(cfg.owner_other, base.owner_other)?,
            group_fg: resolve_opt_color(cfg.group_fg, base.group_fg)?,
            date_fg: resolve_opt_color(cfg.date_fg, base.date_fg)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_cycle_round_trips() {
        let start = ThemeName::EzaDark;
        let mut current = start;
        for _ in 0..ThemeName::BUILTIN.len() {
            current = current.next(false);
        }
        assert_eq!(current, start);

        for theme in ThemeName::BUILTIN {
            assert_eq!(theme.next(false).prev(false), theme);
            assert_eq!(theme.palette().name, theme);
        }

        // Cycling with a custom theme includes Custom after SolarizedLight
        let mut custom_cycle = ThemeName::Custom;
        for _ in 0..=ThemeName::BUILTIN.len() {
            custom_cycle = custom_cycle.next(true);
        }
        assert_eq!(custom_cycle, ThemeName::Custom);
        assert_eq!(ThemeName::SolarizedLight.next(true), ThemeName::Custom);
        assert_eq!(ThemeName::Custom.prev(true), ThemeName::SolarizedLight);
        assert_eq!(ThemeName::EzaDark.prev(true), ThemeName::Custom);
    }

    #[test]
    fn parses_custom_theme_toml_colors() {
        let toml_str = r##"
            bg = "#112233"
            fg = "445566"
            accent = [10, 20, 30]
            border = "rgb(100, 110, 120)"
            dir_fg = "light-blue"
            exec_fg = "green"
            muted = "reset"
        "##;

        let theme = Theme::from_toml_str(toml_str).expect("valid custom theme");
        assert_eq!(theme.name, ThemeName::Custom);
        assert_eq!(theme.bg, Color::Rgb(0x11, 0x22, 0x33));
        assert_eq!(theme.fg, Color::Rgb(0x44, 0x55, 0x66));
        assert_eq!(theme.accent, Color::Rgb(10, 20, 30));
        assert_eq!(theme.border, Color::Rgb(100, 110, 120));
        assert_eq!(theme.dir_fg, Color::LightBlue);
        assert_eq!(theme.exec_fg, Color::Green);
        assert_eq!(theme.muted, Color::Reset);
        // Unspecified fields fall back to EzaDark
        assert_eq!(theme.code_fg, ThemeName::EzaDark.palette().code_fg);
    }
}
