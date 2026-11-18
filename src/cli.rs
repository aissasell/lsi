use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use serde::Deserialize;

use crate::theme::ThemeName;

/// Field used to sort directory entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SortField {
    #[default]
    Name,
    Size,
    Modified,
    #[serde(alias = "ext")]
    Extension,
    Type,
}

impl SortField {
    /// Human-readable label for the sort field.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Size => "size",
            Self::Modified => "modified",
            Self::Extension => "ext",
            Self::Type => "type",
        }
    }

    /// Cycles to the next sort field.
    pub const fn next(self) -> Self {
        match self {
            Self::Name => Self::Size,
            Self::Size => Self::Modified,
            Self::Modified => Self::Extension,
            Self::Extension => Self::Type,
            Self::Type => Self::Name,
        }
    }
}

/// Interactive eza-inspired terminal file explorer.
#[derive(Debug, Clone, Parser)]
#[command(
    name = "lsi",
    version,
    about = "Interactive eza-like terminal file explorer with Nerd Fonts and themes"
)]
pub struct Args {
    /// Starting directory or file path (defaults to current working directory)
    pub path: Option<PathBuf>,

    /// Show hidden files and folders (dotfiles)
    #[arg(short = 'a', long = "all", visible_alias = "hidden")]
    pub all: bool,

    /// Enable all detailed metadata columns (permissions, ownership, time, size)
    #[arg(short = 'l', long = "long")]
    pub long: bool,

    /// Show file permissions column (e.g. drwxr-xr-x)
    #[arg(short = 'p', long = "permissions")]
    pub permissions: bool,

    /// Show owner and group columns
    #[arg(short = 'o', long = "ownership", visible_alias = "user")]
    pub ownership: bool,

    /// Show modification date and time column
    #[arg(short = 't', long = "time", visible_alias = "date")]
    pub time: bool,

    /// Show file size column (enabled by default unless --no-size is passed)
    #[arg(short = 's', long = "size", overrides_with = "no_size")]
    pub size: bool,

    /// Hide the file size column initially
    #[arg(long = "no-size", overrides_with = "size")]
    pub no_size: bool,

    /// Show Nerd Font icons (enabled by default unless --no-icons or config disables it)
    #[arg(long = "icons", overrides_with = "no_icons")]
    pub icons: bool,

    /// Disable Nerd Font icons
    #[arg(long = "no-icons", overrides_with = "icons")]
    pub no_icons: bool,

    /// Show borders around the table and preview panes (enabled by default)
    #[arg(long = "borders", overrides_with = "no_borders")]
    pub borders: bool,

    /// Hide borders around the table and preview panes
    #[arg(long = "no-borders", overrides_with = "borders")]
    pub no_borders: bool,

    /// Show footer status and keybind hint bar (enabled by default)
    #[arg(long = "footer", overrides_with = "no_footer")]
    pub footer: bool,

    /// Hide the footer status and keybind hint bar
    #[arg(long = "no-footer", overrides_with = "footer")]
    pub no_footer: bool,

    /// Initial color theme (overrides config.toml; defaults to custom if theme.toml exists, else eza-dark)
    #[arg(long = "theme", value_enum)]
    pub theme: Option<ThemeName>,

    /// Initial sort field (overrides config.toml; defaults to name)
    #[arg(long = "sort", value_enum)]
    pub sort: Option<SortField>,

    /// Reverse sort order
    #[arg(short = 'r', long = "reverse")]
    pub reverse: bool,

    /// Group directories before files (enabled by default unless --no-dirs-first or config disables it)
    #[arg(long = "dirs-first", overrides_with = "no_dirs_first")]
    pub dirs_first_flag: bool,

    /// Do not group directories before files
    #[arg(long = "no-dirs-first", overrides_with = "dirs_first_flag")]
    pub no_dirs_first: bool,

    /// Start with the side-by-side preview pane open
    #[arg(long = "preview")]
    pub preview: bool,

    /// Maximum number of inline terminal rows to use (dynamically sizes up to N rows; default 20)
    #[arg(short = 'n', long = "max-rows", visible_alias = "height")]
    pub max_rows: Option<u16>,

    /// Write the final working directory to this file when exiting with 'q' (for shell integration)
    #[arg(long = "cwd-file", value_name = "PATH")]
    pub cwd_file: Option<PathBuf>,
}

impl Args {
    /// Effective CLI state for the permissions column.
    #[allow(dead_code)]
    pub const fn show_permissions(&self) -> bool {
        self.long || self.permissions
    }

    /// Effective CLI state for the ownership columns.
    #[allow(dead_code)]
    pub const fn show_ownership(&self) -> bool {
        self.long || self.ownership
    }

    /// Effective CLI state for the modification timestamp column.
    #[allow(dead_code)]
    pub const fn show_time(&self) -> bool {
        self.long || self.time
    }

    /// Effective CLI state for the file size column (when explicitly toggled on CLI).
    #[allow(dead_code)]
    pub const fn show_size(&self) -> bool {
        self.long || !self.no_size
    }

    /// Explicit CLI override for Nerd Font icons, if provided.
    pub const fn show_icons_override(&self) -> Option<bool> {
        if self.no_icons {
            Some(false)
        } else if self.icons {
            Some(true)
        } else {
            None
        }
    }

    /// Explicit CLI override for borders, if provided.
    pub const fn show_borders_override(&self) -> Option<bool> {
        if self.no_borders {
            Some(false)
        } else if self.borders {
            Some(true)
        } else {
            None
        }
    }

    /// Explicit CLI override for footer, if provided.
    pub const fn show_footer_override(&self) -> Option<bool> {
        if self.no_footer {
            Some(false)
        } else if self.footer {
            Some(true)
        } else {
            None
        }
    }

    /// Explicit CLI override for directories-first grouping, if provided.
    pub const fn dirs_first_override(&self) -> Option<bool> {
        if self.no_dirs_first {
            Some(false)
        } else if self.dirs_first_flag {
            Some(true)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_defaults() {
        let args = Args::try_parse_from(["lsi"]).expect("default args parse");
        assert!(args.path.is_none());
        assert!(!args.all);
        assert!(!args.show_permissions());
        assert!(!args.show_ownership());
        assert!(!args.show_time());
        assert!(args.show_size());
        assert_eq!(args.show_icons_override(), None);
        assert_eq!(args.show_borders_override(), None);
        assert_eq!(args.show_footer_override(), None);
        assert_eq!(args.theme, None);
        assert_eq!(args.sort, None);
        assert!(!args.reverse);
        assert_eq!(args.dirs_first_override(), None);
        assert!(!args.preview);
        assert_eq!(args.max_rows, None);
    }

    #[test]
    fn parses_metadata_and_theme_flags() {
        let args = Args::try_parse_from([
            "lsi",
            "-a",
            "-p",
            "-o",
            "-t",
            "--theme",
            "catppuccin",
            "--sort",
            "size",
            "-r",
            "--no-dirs-first",
            "--no-icons",
            "--no-borders",
            "--no-footer",
            "--preview",
            "/tmp",
        ])
        .expect("flags parse");

        assert!(args.all);
        assert!(args.show_permissions());
        assert!(args.show_ownership());
        assert!(args.show_time());
        assert!(args.show_size());
        assert_eq!(args.show_icons_override(), Some(false));
        assert_eq!(args.show_borders_override(), Some(false));
        assert_eq!(args.show_footer_override(), Some(false));
        assert_eq!(args.theme, Some(ThemeName::Catppuccin));
        assert_eq!(args.sort, Some(SortField::Size));
        assert!(args.reverse);
        assert_eq!(args.dirs_first_override(), Some(false));
        assert!(args.preview);
        assert_eq!(args.path, Some(PathBuf::from("/tmp")));
    }

    #[test]
    fn long_flag_enables_all_columns() {
        let args = Args::try_parse_from(["lsi", "-l"]).expect("-l parses");
        assert!(args.show_permissions());
        assert!(args.show_ownership());
        assert!(args.show_time());
        assert!(args.show_size());
    }

    #[test]
    fn hidden_alias_and_no_size_work() {
        let args = Args::try_parse_from(["lsi", "--hidden", "--no-size"]).expect("aliases parse");
        assert!(args.all);
        assert!(!args.show_size());

        let args_override = Args::try_parse_from(["lsi", "--no-size", "--size"]).expect("override");
        assert!(args_override.show_size());
    }

    #[test]
    fn parses_max_rows_flags_and_mutually_overriding_toggles() {
        let short_args = Args::try_parse_from(["lsi", "-n", "12"]).expect("-n parses");
        assert_eq!(short_args.max_rows, Some(12));

        let long_args =
            Args::try_parse_from(["lsi", "--max-rows", "15"]).expect("--max-rows parses");
        assert_eq!(long_args.max_rows, Some(15));

        let toggle_args = Args::try_parse_from([
            "lsi",
            "--no-borders",
            "--borders",
            "--no-footer",
            "--footer",
            "--no-icons",
            "--icons",
            "--no-dirs-first",
            "--dirs-first",
        ])
        .expect("mutually overriding toggles parse");
        assert_eq!(toggle_args.show_borders_override(), Some(true));
        assert_eq!(toggle_args.show_footer_override(), Some(true));
        assert_eq!(toggle_args.show_icons_override(), Some(true));
        assert_eq!(toggle_args.dirs_first_override(), Some(true));
        assert_eq!(toggle_args.cwd_file, None);

        let cwd_args =
            Args::try_parse_from(["lsi", "--cwd-file", "/tmp/lsi-cwd"]).expect("--cwd-file parses");
        assert_eq!(cwd_args.cwd_file, Some(PathBuf::from("/tmp/lsi-cwd")));
    }
}
