use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::cli::SortField;
use crate::theme::{Theme, ThemeName};

/// User configuration loaded from `$HOME/.config/lsi/config.toml`.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct ConfigFile {
    #[serde(alias = "borders")]
    pub show_borders: Option<bool>,
    #[serde(alias = "footer")]
    pub show_footer: Option<bool>,
    #[serde(alias = "all", alias = "hidden")]
    pub show_hidden: Option<bool>,
    pub long: Option<bool>,
    #[serde(alias = "permissions")]
    pub show_permissions: Option<bool>,
    #[serde(alias = "ownership", alias = "user")]
    pub show_ownership: Option<bool>,
    #[serde(alias = "time", alias = "date")]
    pub show_time: Option<bool>,
    #[serde(alias = "size")]
    pub show_size: Option<bool>,
    #[serde(alias = "icons")]
    pub show_icons: Option<bool>,
    #[serde(alias = "preview")]
    pub show_preview: Option<bool>,
    pub dirs_first: Option<bool>,
    #[serde(alias = "sort_reverse")]
    pub reverse: Option<bool>,
    #[serde(alias = "sort_field")]
    pub sort: Option<SortField>,
    pub theme: Option<ThemeName>,
    #[serde(alias = "height")]
    pub max_rows: Option<u16>,
}

/// Combined result of loading `config.toml` and `theme.toml`.
#[derive(Debug, Default, Clone)]
pub struct LoadedConfig {
    pub config: ConfigFile,
    pub custom_theme: Option<Theme>,
    pub warning: Option<String>,
}

/// Returns `$HOME/.config/lsi` when `HOME` is set.
pub fn default_config_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config").join("lsi"))
}

/// Loads `config.toml` and `theme.toml` from `dir`, recording non-fatal warnings on parse errors.
pub fn load_from_dir(dir: &Path) -> LoadedConfig {
    let mut warnings = Vec::new();

    let config_path = dir.join("config.toml");
    let config = if config_path.is_file() {
        match fs::read_to_string(&config_path) {
            Ok(raw) => match toml::from_str::<ConfigFile>(&raw) {
                Ok(parsed) => parsed,
                Err(err) => {
                    warnings.push(format!("config.toml: {err}"));
                    ConfigFile::default()
                }
            },
            Err(err) => {
                warnings.push(format!("config.toml: {err}"));
                ConfigFile::default()
            }
        }
    } else {
        ConfigFile::default()
    };

    let theme_path = dir.join("theme.toml");
    let custom_theme = if theme_path.is_file() {
        match fs::read_to_string(&theme_path) {
            Ok(raw) => match Theme::from_toml_str(&raw) {
                Ok(theme) => Some(theme),
                Err(err) => {
                    warnings.push(format!("theme.toml: {err}"));
                    None
                }
            },
            Err(err) => {
                warnings.push(format!("theme.toml: {err}"));
                None
            }
        }
    } else {
        None
    };

    let warning = if warnings.is_empty() {
        None
    } else {
        Some(warnings.join("; "))
    };

    LoadedConfig {
        config,
        custom_theme,
        warning,
    }
}

/// Loads user configuration and custom theme from `$HOME/.config/lsi`.
pub fn load_user_config() -> LoadedConfig {
    let Some(dir) = default_config_dir() else {
        return LoadedConfig::default();
    };
    load_from_dir(&dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_config_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lsi_cfg_test_{tag}_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn loads_config_and_custom_theme_from_dir() {
        let dir = make_temp_config_dir("valid");
        fs::write(
            dir.join("config.toml"),
            r#"
show_borders = false
show_footer = false
show_hidden = true
show_permissions = true
show_ownership = true
show_time = true
show_size = false
show_icons = false
show_preview = true
dirs_first = false
reverse = true
sort = "modified"
max_rows = 28
"#,
        )
        .unwrap();

        fs::write(
            dir.join("theme.toml"),
            r##"
bg = "#101214"
accent = "#ff9900"
dir_fg = [100, 150, 200]
"##,
        )
        .unwrap();

        let loaded = load_from_dir(&dir);
        assert!(loaded.warning.is_none());
        assert_eq!(loaded.config.show_borders, Some(false));
        assert_eq!(loaded.config.show_footer, Some(false));
        assert_eq!(loaded.config.show_hidden, Some(true));
        assert_eq!(loaded.config.show_permissions, Some(true));
        assert_eq!(loaded.config.show_ownership, Some(true));
        assert_eq!(loaded.config.show_time, Some(true));
        assert_eq!(loaded.config.show_size, Some(false));
        assert_eq!(loaded.config.show_icons, Some(false));
        assert_eq!(loaded.config.show_preview, Some(true));
        assert_eq!(loaded.config.dirs_first, Some(false));
        assert_eq!(loaded.config.reverse, Some(true));
        assert_eq!(loaded.config.sort, Some(SortField::Modified));
        assert_eq!(loaded.config.max_rows, Some(28));

        let custom = loaded.custom_theme.expect("custom theme parsed");
        assert_eq!(custom.name, ThemeName::Custom);
        assert_eq!(custom.bg, Color::Rgb(0x10, 0x12, 0x14));
        assert_eq!(custom.accent, Color::Rgb(0xff, 0x99, 0x00));
        assert_eq!(custom.dir_fg, Color::Rgb(100, 150, 200));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_toml_produces_warning_without_failing() {
        let dir = make_temp_config_dir("invalid");
        fs::write(dir.join("config.toml"), "show_borders = [not a bool").unwrap();
        fs::write(dir.join("theme.toml"), "bg = \"not-a-color\"").unwrap();

        let loaded = load_from_dir(&dir);
        let warning = loaded.warning.expect("warning recorded");
        assert!(warning.contains("config.toml"));
        assert!(warning.contains("theme.toml"));
        assert!(loaded.custom_theme.is_none());

        let _ = fs::remove_dir_all(&dir);
    }
}
