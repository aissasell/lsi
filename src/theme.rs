use clap::ValueEnum;
use ratatui::style::Color;

/// Built-in color themes supported by `lsi`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum ThemeName {
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
    /// All available themes in cycling order.
    #[allow(dead_code)]
    pub const ALL: [Self; 7] = [
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
    pub const fn next(self) -> Self {
        match self {
            Self::EzaDark => Self::Catppuccin,
            Self::Catppuccin => Self::Dracula,
            Self::Dracula => Self::Nord,
            Self::Nord => Self::Gruvbox,
            Self::Gruvbox => Self::TokyoNight,
            Self::TokyoNight => Self::SolarizedLight,
            Self::SolarizedLight => Self::EzaDark,
        }
    }

    /// Returns the previous theme in the cycle.
    #[allow(dead_code)]
    pub const fn prev(self) -> Self {
        match self {
            Self::EzaDark => Self::SolarizedLight,
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
            Self::EzaDark => Theme {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_cycle_round_trips() {
        let start = ThemeName::EzaDark;
        let mut current = start;
        for _ in 0..ThemeName::ALL.len() {
            current = current.next();
        }
        assert_eq!(current, start);

        for theme in ThemeName::ALL {
            assert_eq!(theme.next().prev(), theme);
            assert_eq!(theme.palette().name, theme);
        }
    }
}
