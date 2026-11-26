use std::fs;
use std::path::{Component, Path, PathBuf};
#[cfg(feature = "syntax-highlight")]
use std::sync::Arc;

use serde::Deserialize;
#[cfg(feature = "syntax-highlight")]
use syntect::highlighting::Theme as SyntectTheme;
#[cfg(feature = "syntax-highlight")]
use two_face::theme::EmbeddedThemeName;

use crate::cli::SortField;
use crate::keybinds::Keybinds;
#[cfg(feature = "syntax-highlight")]
use crate::preview::{load_custom_syntax_theme, parse_embedded_syntax_theme};
use crate::preview::{ImageProtocol, SyntaxThemeSelection};
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
    pub syntax_highlight: Option<bool>,
    pub syntax_theme: Option<String>,
    pub image_preview: Option<bool>,
    pub video_preview: Option<bool>,
    pub image_protocol: Option<ImageProtocol>,
    pub preview_max_file_mb: Option<u64>,
    pub dirs_first: Option<bool>,
    #[serde(alias = "sort_reverse")]
    pub reverse: Option<bool>,
    #[serde(alias = "sort_field")]
    pub sort: Option<SortField>,
    pub theme: Option<String>,
    #[serde(alias = "height")]
    pub max_rows: Option<u16>,
}

/// Outcome of resolving a requested theme name against built-ins and `$HOME/.config/lsi/themes`.
#[derive(Debug, Clone)]
pub enum ThemeLoadOutcome {
    Builtin(ThemeName),
    Custom { name: String, theme: Theme },
    NotFound { name: String },
    Invalid { error: String },
}

/// Outcome of resolving a requested syntax highlighting theme against `themes/*.tmTheme` and `two-face`.
#[cfg(feature = "syntax-highlight")]
#[derive(Debug, Clone)]
pub enum SyntaxThemeLoadOutcome {
    Auto,
    Embedded(EmbeddedThemeName),
    Custom {
        name: String,
        theme: Arc<SyntectTheme>,
    },
    NotFound {
        name: String,
    },
    Invalid {
        error: String,
    },
}

/// Combined result of loading `config.toml`, named theme from `themes/`, and `keybinds.toml`.
#[derive(Debug, Default, Clone)]
pub struct LoadedConfig {
    pub config: ConfigFile,
    pub custom_theme: Option<Theme>,
    pub custom_theme_name: Option<String>,
    pub syntax_theme: SyntaxThemeSelection,
    pub themes_dir: Option<PathBuf>,
    pub keybinds: Keybinds,
    pub warning: Option<String>,
}

/// Returns `$HOME/.config/lsi` (or `%USERPROFILE%\.config\lsi` on Windows) when set.
pub fn default_config_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .or_else(|| std::env::var_os("USERPROFILE").filter(|v| !v.is_empty()))
        .map(|home| PathBuf::from(home).join(".config").join("lsi"))
}

fn validate_single_component_name<'a>(raw: &'a str, kind: &str) -> Result<&'a str, String> {
    let name = raw.trim();
    if name.is_empty() || name.contains('/') || name.contains('\\') || name == "." || name == ".." {
        return Err(format!("Invalid {kind} '{name}'"));
    }

    let mut components = Path::new(name).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(name),
        _ => Err(format!("Invalid {kind} '{name}'")),
    }
}

/// Resolves `raw_name` against built-in themes first, then `<themes_dir>/<name>.toml`.
///
/// Built-in names always take precedence so files like `themes/catppuccin.toml` are ignored.
pub fn load_named_theme(themes_dir: &Path, raw_name: &str) -> ThemeLoadOutcome {
    let name = match validate_single_component_name(raw_name, "theme name") {
        Ok(name) => name,
        Err(error) => return ThemeLoadOutcome::Invalid { error },
    };
    if let Some(builtin) = ThemeName::parse_builtin(name) {
        return ThemeLoadOutcome::Builtin(builtin);
    }

    let path = themes_dir.join(format!("{name}.toml"));
    if !path.is_file() {
        return ThemeLoadOutcome::NotFound {
            name: name.to_string(),
        };
    }

    match fs::read_to_string(&path) {
        Ok(raw) => match Theme::from_toml_str(&raw) {
            Ok(theme) => ThemeLoadOutcome::Custom {
                name: name.to_string(),
                theme,
            },
            Err(err) => ThemeLoadOutcome::Invalid {
                error: format!("themes/{name}.toml: {err}"),
            },
        },
        Err(err) => ThemeLoadOutcome::Invalid {
            error: format!("themes/{name}.toml: {err}"),
        },
    }
}

#[cfg(feature = "syntax-highlight")]
fn find_custom_tmtheme_path(themes_dir: &Path, stem: &str) -> Option<PathBuf> {
    let direct = themes_dir.join(format!("{stem}.tmTheme"));
    if direct.is_file() {
        return Some(direct);
    }

    let entries = fs::read_dir(themes_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("tmtheme"))
            && path
                .file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.eq_ignore_ascii_case(stem))
        {
            return Some(path);
        }
    }

    None
}

/// Resolves `raw_name` against `"auto"`, `<themes_dir>/<name>.tmTheme`, and `two-face` built-ins.
///
/// Custom `.tmTheme` files take precedence over embedded `two-face` themes of the same name.
#[cfg(feature = "syntax-highlight")]
pub fn load_named_syntax_theme(themes_dir: &Path, raw_name: &str) -> SyntaxThemeLoadOutcome {
    let trimmed = match validate_single_component_name(raw_name, "syntax theme name") {
        Ok(name) => name,
        Err(error) => return SyntaxThemeLoadOutcome::Invalid { error },
    };
    if trimmed.eq_ignore_ascii_case("auto") {
        return SyntaxThemeLoadOutcome::Auto;
    }

    let stem = trimmed
        .get(..trimmed.len().saturating_sub(8))
        .filter(|_| trimmed.len() >= 8 && trimmed[trimmed.len() - 8..].eq_ignore_ascii_case(".tmtheme"))
        .unwrap_or(trimmed);
    if stem.is_empty() || stem == "." || stem == ".." {
        return SyntaxThemeLoadOutcome::Invalid {
            error: format!("Invalid syntax theme name '{trimmed}'"),
        };
    }

    if let Some(path) = find_custom_tmtheme_path(themes_dir, stem) {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(stem)
            .to_string();
        return match load_custom_syntax_theme(&path) {
            Ok(theme) => SyntaxThemeLoadOutcome::Custom {
                name: stem.to_string(),
                theme,
            },
            Err(err) => SyntaxThemeLoadOutcome::Invalid {
                error: format!("themes/{file_name}: {err}"),
            },
        };
    }

    if let Some(embedded) = parse_embedded_syntax_theme(stem) {
        SyntaxThemeLoadOutcome::Embedded(embedded)
    } else {
        SyntaxThemeLoadOutcome::NotFound {
            name: stem.to_string(),
        }
    }
}

/// Loads `config.toml`, optional named theme from `themes/`, and `keybinds.toml` from `dir`.
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

    let themes_dir = dir.join("themes");
    let (custom_theme, custom_theme_name) = if let Some(ref requested) = config.theme {
        match load_named_theme(&themes_dir, requested) {
            ThemeLoadOutcome::Builtin(_) => (None, None),
            ThemeLoadOutcome::Custom { name, theme } => (Some(theme), Some(name)),
            ThemeLoadOutcome::NotFound { name } => {
                warnings.push(format!(
                    "theme '{name}' not found in themes/; falling back to eza-dark"
                ));
                (None, None)
            }
            ThemeLoadOutcome::Invalid { error } => {
                warnings.push(error);
                (None, None)
            }
        }
    } else {
        (None, None)
    };

    #[cfg(feature = "syntax-highlight")]
    let syntax_theme = if let Some(ref requested) = config.syntax_theme {
        match load_named_syntax_theme(&themes_dir, requested) {
            SyntaxThemeLoadOutcome::Auto => SyntaxThemeSelection::Auto,
            SyntaxThemeLoadOutcome::Embedded(name) => SyntaxThemeSelection::Embedded(name),
            SyntaxThemeLoadOutcome::Custom { name, theme } => {
                SyntaxThemeSelection::Custom { name, theme }
            }
            SyntaxThemeLoadOutcome::NotFound { name } => {
                warnings.push(format!(
                    "syntax theme '{name}' not found; falling back to auto"
                ));
                SyntaxThemeSelection::Auto
            }
            SyntaxThemeLoadOutcome::Invalid { error } => {
                warnings.push(error);
                SyntaxThemeSelection::Auto
            }
        }
    } else {
        SyntaxThemeSelection::Auto
    };
    #[cfg(not(feature = "syntax-highlight"))]
    let syntax_theme = SyntaxThemeSelection::Auto;

    let keybinds_path = dir.join("keybinds.toml");
    let keybinds = if keybinds_path.is_file() {
        match fs::read_to_string(&keybinds_path) {
            Ok(raw) => match Keybinds::from_toml_str(&raw) {
                Ok(parsed) => parsed,
                Err(err) => {
                    warnings.push(format!("keybinds.toml: {err}"));
                    Keybinds::default()
                }
            },
            Err(err) => {
                warnings.push(format!("keybinds.toml: {err}"));
                Keybinds::default()
            }
        }
    } else {
        Keybinds::default()
    };

    let warning = if warnings.is_empty() {
        None
    } else {
        Some(warnings.join("; "))
    };

    LoadedConfig {
        config,
        custom_theme,
        custom_theme_name,
        syntax_theme,
        themes_dir: Some(themes_dir),
        keybinds,
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
syntax_highlight = false
image_preview = false
video_preview = false
image_protocol = "halfblocks"
preview_max_file_mb = 12
dirs_first = false
reverse = true
sort = "modified"
theme = "catppuccin-mocha-mauve"
max_rows = 28
"#,
        )
        .unwrap();

        fs::create_dir_all(dir.join("themes")).unwrap();
        fs::write(
            dir.join("themes/catppuccin-mocha-mauve.toml"),
            r##"
bg = "#101214"
accent = "#ff9900"
dir_fg = [100, 150, 200]
"##,
        )
        .unwrap();
        // Should be ignored because "catppuccin" is a built-in theme name.
        fs::write(dir.join("themes/catppuccin.toml"), "bg = \"#010203\"\n").unwrap();

        fs::write(
            dir.join("keybinds.toml"),
            r#"
rename_item = "F2"
quit_cd = ["q", "Ctrl+w"]
"#,
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
        assert_eq!(loaded.config.syntax_highlight, Some(false));
        assert_eq!(loaded.config.image_preview, Some(false));
        assert_eq!(loaded.config.video_preview, Some(false));
        assert_eq!(
            loaded.config.image_protocol,
            Some(ImageProtocol::Halfblocks)
        );
        assert_eq!(loaded.config.preview_max_file_mb, Some(12));
        assert_eq!(loaded.config.dirs_first, Some(false));
        assert_eq!(loaded.config.reverse, Some(true));
        assert_eq!(loaded.config.sort, Some(SortField::Modified));
        assert_eq!(
            loaded.config.theme.as_deref(),
            Some("catppuccin-mocha-mauve")
        );
        assert_eq!(loaded.config.max_rows, Some(28));

        let custom = loaded.custom_theme.expect("custom theme parsed");
        assert_eq!(custom.name, ThemeName::Custom);
        assert_eq!(
            loaded.custom_theme_name.as_deref(),
            Some("catppuccin-mocha-mauve")
        );
        assert_eq!(custom.bg, Color::Rgb(0x10, 0x12, 0x14));
        assert_eq!(custom.accent, Color::Rgb(0xff, 0x99, 0x00));
        assert_eq!(custom.dir_fg, Color::Rgb(100, 150, 200));
        assert_eq!(custom.code_fg, ThemeName::EzaDark.palette().code_fg);

        assert_eq!(
            loaded.keybinds.rename_item,
            vec![crate::keybinds::KeyCombo::parse("F2").unwrap()]
        );
        assert_eq!(
            loaded.keybinds.quit_cd,
            vec![
                crate::keybinds::KeyCombo::parse("q").unwrap(),
                crate::keybinds::KeyCombo::parse("Ctrl+w").unwrap()
            ]
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn builtin_theme_name_ignores_file_in_themes_dir() {
        let dir = make_temp_config_dir("builtin-ignore");
        fs::create_dir_all(dir.join("themes")).unwrap();
        fs::write(dir.join("themes/catppuccin.toml"), "bg = \"#abcdef\"\n").unwrap();
        fs::write(dir.join("config.toml"), "theme = \"catppuccin\"\n").unwrap();

        let loaded = load_from_dir(&dir);
        assert!(loaded.warning.is_none());
        assert!(loaded.custom_theme.is_none());
        assert!(loaded.custom_theme_name.is_none());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_custom_theme_records_warning_and_falls_back() {
        let dir = make_temp_config_dir("missing-theme");
        fs::write(dir.join("config.toml"), "theme = \"missing-theme\"\n").unwrap();

        let loaded = load_from_dir(&dir);
        let warning = loaded.warning.expect("missing theme warning");
        assert!(warning.contains("missing-theme"));
        assert!(loaded.custom_theme.is_none());
        assert!(loaded.custom_theme_name.is_none());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_toml_produces_warning_without_failing() {
        let dir = make_temp_config_dir("invalid");
        fs::create_dir_all(dir.join("themes")).unwrap();
        fs::write(dir.join("config.toml"), "show_borders = [not a bool").unwrap();
        fs::write(dir.join("themes/broken-theme.toml"), "bg = \"not-a-color\"").unwrap();
        fs::write(dir.join("keybinds.toml"), "rename_item = \"NotAKey\"").unwrap();

        let loaded = load_from_dir(&dir);
        let warning = loaded.warning.expect("warning recorded");
        assert!(warning.contains("config.toml"));
        assert!(warning.contains("keybinds.toml"));
        assert!(loaded.custom_theme.is_none());
        assert_eq!(loaded.keybinds, Keybinds::default());

        match load_named_theme(&dir.join("themes"), "broken-theme") {
            ThemeLoadOutcome::Invalid { error } => assert!(error.contains("themes/broken-theme.toml")),
            other => panic!("expected invalid theme outcome, got {other:?}"),
        }
        match load_named_theme(&dir.join("themes"), "../escape") {
            ThemeLoadOutcome::Invalid { error } => assert!(error.contains("Invalid theme name")),
            other => panic!("expected path traversal rejection, got {other:?}"),
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(feature = "syntax-highlight")]
    #[test]
    fn loads_embedded_and_custom_tmtheme_syntax_themes_with_warnings() {
        let dir = make_temp_config_dir("syntax-theme");
        let themes_dir = dir.join("themes");
        fs::create_dir_all(&themes_dir).unwrap();

        let valid_tmtheme = r##"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>name</key>
  <string>CustomSyntax</string>
  <key>settings</key>
  <array>
    <dict>
      <key>settings</key>
      <dict>
        <key>foreground</key>
        <string>#123456</string>
      </dict>
    </dict>
  </array>
</dict>
</plist>
"##;

        // 1. "auto" resolves without warnings
        fs::write(dir.join("config.toml"), "syntax_theme = \"auto\"\n").unwrap();
        let loaded_auto = load_from_dir(&dir);
        assert!(loaded_auto.warning.is_none());
        assert_eq!(loaded_auto.syntax_theme, SyntaxThemeSelection::Auto);

        // 2. Embedded name resolves case-/punctuation-insensitively
        fs::write(
            dir.join("config.toml"),
            "syntax_theme = \"catppuccin-mocha\"\n",
        )
        .unwrap();
        let loaded_embedded = load_from_dir(&dir);
        assert!(loaded_embedded.warning.is_none());
        assert_eq!(
            loaded_embedded.syntax_theme,
            SyntaxThemeSelection::Embedded(EmbeddedThemeName::CatppuccinMocha)
        );

        // 3. Custom .tmTheme in themes/ loads cleanly
        fs::write(themes_dir.join("custom-purple.tmTheme"), valid_tmtheme).unwrap();
        fs::write(dir.join("config.toml"), "syntax_theme = \"custom-purple\"\n").unwrap();
        let loaded_custom = load_from_dir(&dir);
        assert!(loaded_custom.warning.is_none());
        assert!(matches!(
            loaded_custom.syntax_theme,
            SyntaxThemeSelection::Custom { ref name, .. } if name == "custom-purple"
        ));

        // 4. Custom .tmTheme overrides an embedded two-face theme of the same name
        fs::write(themes_dir.join("Dracula.tmTheme"), valid_tmtheme).unwrap();
        fs::write(dir.join("config.toml"), "syntax_theme = \"Dracula\"\n").unwrap();
        let loaded_override = load_from_dir(&dir);
        assert!(loaded_override.warning.is_none());
        assert!(matches!(
            loaded_override.syntax_theme,
            SyntaxThemeSelection::Custom { ref name, .. } if name == "Dracula"
        ));

        // 5. Missing theme produces warning and falls back to Auto
        fs::write(
            dir.join("config.toml"),
            "syntax_theme = \"nonexistent-syntax\"\n",
        )
        .unwrap();
        let loaded_missing = load_from_dir(&dir);
        let missing_warn = loaded_missing.warning.expect("missing syntax warning");
        assert!(missing_warn.contains("nonexistent-syntax"));
        assert_eq!(loaded_missing.syntax_theme, SyntaxThemeSelection::Auto);

        // 6. Malformed .tmTheme produces warning and falls back to Auto
        fs::write(themes_dir.join("broken.tmTheme"), "not valid plist xml").unwrap();
        fs::write(dir.join("config.toml"), "syntax_theme = \"broken\"\n").unwrap();
        let loaded_broken = load_from_dir(&dir);
        let broken_warn = loaded_broken.warning.expect("broken tmTheme warning");
        assert!(broken_warn.contains("themes/broken.tmTheme"));
        assert_eq!(loaded_broken.syntax_theme, SyntaxThemeSelection::Auto);

        // 7. Path traversal is rejected
        match load_named_syntax_theme(&themes_dir, "../evil") {
            SyntaxThemeLoadOutcome::Invalid { error } => {
                assert!(error.contains("Invalid syntax theme name"));
            }
            other => panic!("expected path traversal rejection, got {other:?}"),
        }

        let _ = fs::remove_dir_all(&dir);
    }
}
