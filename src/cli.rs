use std::path::PathBuf;

use clap::{Parser, ValueEnum};

use crate::theme::ThemeName;

/// Field used to sort directory entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum SortField {
    #[default]
    Name,
    Size,
    Modified,
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

    /// Disable Nerd Font icons
    #[arg(long = "no-icons")]
    pub no_icons: bool,

    /// Initial color theme
    #[arg(long = "theme", value_enum, default_value_t = ThemeName::EzaDark)]
    pub theme: ThemeName,

    /// Initial sort field
    #[arg(long = "sort", value_enum, default_value_t = SortField::Name)]
    pub sort: SortField,

    /// Reverse sort order
    #[arg(short = 'r', long = "reverse")]
    pub reverse: bool,

    /// Do not group directories before files
    #[arg(long = "no-dirs-first")]
    pub no_dirs_first: bool,

    /// Start with the side-by-side preview pane open
    #[arg(long = "preview")]
    pub preview: bool,

    /// Maximum number of inline terminal rows to use (dynamically sizes up to N rows)
    #[arg(short = 'n', long = "max-rows", visible_alias = "height", default_value_t = 20)]
    pub max_rows: u16,
}

impl Args {
    /// Effective initial state for the permissions column.
    pub const fn show_permissions(&self) -> bool {
        self.long || self.permissions
    }

    /// Effective initial state for the ownership columns.
    pub const fn show_ownership(&self) -> bool {
        self.long || self.ownership
    }

    /// Effective initial state for the modification timestamp column.
    pub const fn show_time(&self) -> bool {
        self.long || self.time
    }

    /// Effective initial state for the file size column.
    pub const fn show_size(&self) -> bool {
        self.long || !self.no_size
    }

    /// Effective initial state for Nerd Font icons.
    pub const fn show_icons(&self) -> bool {
        !self.no_icons
    }

    /// Effective initial state for grouping directories first.
    pub const fn dirs_first(&self) -> bool {
        !self.no_dirs_first
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
        assert!(args.show_icons());
        assert_eq!(args.theme, ThemeName::EzaDark);
        assert_eq!(args.sort, SortField::Name);
        assert!(!args.reverse);
        assert!(args.dirs_first());
        assert!(!args.preview);
        assert_eq!(args.max_rows, 20);
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
            "--preview",
            "/tmp",
        ])
        .expect("flags parse");

        assert!(args.all);
        assert!(args.show_permissions());
        assert!(args.show_ownership());
        assert!(args.show_time());
        assert!(args.show_size());
        assert!(!args.show_icons());
        assert_eq!(args.theme, ThemeName::Catppuccin);
        assert_eq!(args.sort, SortField::Size);
        assert!(args.reverse);
        assert!(!args.dirs_first());
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
    fn parses_max_rows_flags() {
        let short_args = Args::try_parse_from(["lsi", "-n", "12"]).expect("-n parses");
        assert_eq!(short_args.max_rows, 12);

        let long_args =
            Args::try_parse_from(["lsi", "--max-rows", "15"]).expect("--max-rows parses");
        assert_eq!(long_args.max_rows, 15);
    }
}
