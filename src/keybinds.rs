use std::collections::HashSet;

use anyhow::{bail, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::Deserialize;

/// Normalized key combination (key code + modifiers) used for keybind matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyCombo {
    /// Normalizes key code and modifier representation across terminals and config syntax.
    pub fn normalize(mut code: KeyCode, modifiers: KeyModifiers) -> Self {
        let mut mods = modifiers & (KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT);
        match code {
            KeyCode::Char(c) => {
                if mods.contains(KeyModifiers::SHIFT) {
                    mods.remove(KeyModifiers::SHIFT);
                    code = KeyCode::Char(c.to_ascii_uppercase());
                } else if mods.contains(KeyModifiers::CONTROL) && c.is_ascii_uppercase() {
                    code = KeyCode::Char(c.to_ascii_lowercase());
                }
            }
            KeyCode::Tab if mods.contains(KeyModifiers::SHIFT) => {
                mods.remove(KeyModifiers::SHIFT);
                code = KeyCode::BackTab;
            }
            KeyCode::BackTab => {
                mods.remove(KeyModifiers::SHIFT);
            }
            _ => {}
        }
        Self {
            code,
            modifiers: mods,
        }
    }

    /// Converts a crossterm `KeyEvent` into a normalized `KeyCombo`.
    pub fn from_event(key: KeyEvent) -> Self {
        Self::normalize(key.code, key.modifiers)
    }

    /// Parses a key specification string such as `"j"`, `"Down"`, `"Ctrl+r"`, or `"Shift+Delete"`.
    pub fn parse(raw: &str) -> Result<Self> {
        if raw == " " {
            return Ok(Self::normalize(KeyCode::Char(' '), KeyModifiers::NONE));
        }

        let trimmed = raw.trim();
        if trimmed.is_empty() {
            bail!("Empty key binding");
        }

        if trimmed.chars().count() == 1 {
            let ch = trimmed.chars().next().expect("single char verified");
            return Ok(Self::normalize(KeyCode::Char(ch), KeyModifiers::NONE));
        }

        const PREFIXES: &[(&str, KeyModifiers)] = &[
            ("ctrl+", KeyModifiers::CONTROL),
            ("control+", KeyModifiers::CONTROL),
            ("alt+", KeyModifiers::ALT),
            ("meta+", KeyModifiers::ALT),
            ("shift+", KeyModifiers::SHIFT),
        ];

        let mut rest = trimmed;
        let mut modifiers = KeyModifiers::NONE;
        loop {
            let mut matched = false;
            for (prefix, flag) in PREFIXES {
                if rest
                    .get(..prefix.len())
                    .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
                {
                    modifiers |= *flag;
                    rest = &rest[prefix.len()..];
                    matched = true;
                    break;
                }
            }
            if !matched {
                break;
            }
        }

        if rest.is_empty() {
            bail!("Missing key name in '{raw}'");
        }

        let code = if rest.chars().count() == 1 {
            KeyCode::Char(rest.chars().next().expect("single char verified"))
        } else {
            match rest.to_ascii_lowercase().as_str() {
                "enter" | "return" | "cr" => KeyCode::Enter,
                "esc" | "escape" => KeyCode::Esc,
                "space" | "spc" => KeyCode::Char(' '),
                "tab" => KeyCode::Tab,
                "backtab" => KeyCode::BackTab,
                "backspace" | "bs" => KeyCode::Backspace,
                "delete" | "del" => KeyCode::Delete,
                "insert" | "ins" => KeyCode::Insert,
                "up" => KeyCode::Up,
                "down" => KeyCode::Down,
                "left" => KeyCode::Left,
                "right" => KeyCode::Right,
                "home" => KeyCode::Home,
                "end" => KeyCode::End,
                "pageup" | "pgup" => KeyCode::PageUp,
                "pagedown" | "pgdn" | "pgdown" => KeyCode::PageDown,
                "plus" => KeyCode::Char('+'),
                "minus" => KeyCode::Char('-'),
                lower if lower.starts_with('f') => {
                    if let Ok(n) = lower[1..].parse::<u8>() {
                        if (1..=24).contains(&n) {
                            KeyCode::F(n)
                        } else {
                            bail!("Function key out of range (F1..F24): '{raw}'");
                        }
                    } else {
                        bail!("Unknown key '{raw}'");
                    }
                }
                _ => bail!("Unknown key '{raw}'"),
            }
        };

        Ok(Self::normalize(code, modifiers))
    }

    /// Formats this key combo concisely for footer hints and the help modal.
    pub fn display_short(&self) -> String {
        let mut out = String::new();
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            out.push_str("Ctrl+");
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            out.push_str("Alt+");
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            out.push_str("Shift+");
        }

        match self.code {
            KeyCode::BackTab => {
                out.push_str("Shift+Tab");
            }
            KeyCode::Char(' ') => out.push_str("Space"),
            KeyCode::Char(c) => {
                if self.modifiers.contains(KeyModifiers::CONTROL) && c.is_ascii_uppercase() {
                    out.push_str("Shift+");
                }
                out.push(c);
            }
            KeyCode::Enter => out.push_str("Enter"),
            KeyCode::Esc => out.push_str("Esc"),
            KeyCode::Tab => out.push_str("Tab"),
            KeyCode::Backspace => out.push_str("Backspace"),
            KeyCode::Delete => out.push_str("Del"),
            KeyCode::Insert => out.push_str("Ins"),
            KeyCode::Up => out.push_str("Up"),
            KeyCode::Down => out.push_str("Down"),
            KeyCode::Left => out.push_str("Left"),
            KeyCode::Right => out.push_str("Right"),
            KeyCode::Home => out.push_str("Home"),
            KeyCode::End => out.push_str("End"),
            KeyCode::PageUp => out.push_str("PgUp"),
            KeyCode::PageDown => out.push_str("PgDn"),
            KeyCode::F(n) => {
                out.push('F');
                out.push_str(&n.to_string());
            }
            other => out.push_str(&format!("{other:?}")),
        }
        out
    }
}

/// Actions dispatchable while in `InputMode::Normal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalAction {
    MoveDown,
    MoveUp,
    PageDown,
    PageUp,
    JumpTop,
    JumpBottom,
    GoParent,
    HistoryBack,
    HistoryForward,
    JumpHome,
    Refresh,
    Open,
    OpenEditor,
    OpenSystem,
    ToggleSelect,
    Yank,
    Cut,
    Paste,
    Trash,
    PermanentDelete,
    CreateItem,
    RenameItem,
    ToggleHidden,
    TogglePermissions,
    ToggleOwnership,
    ToggleTime,
    ToggleSize,
    ToggleIcons,
    ToggleLong,
    ToggleBorders,
    ToggleFooter,
    TogglePreview,
    CycleTheme,
    CycleSort,
    ToggleReverseSort,
    ToggleDirsFirst,
    Search,
    Cancel,
    ToggleHelp,
    QuitCd,
    Quit,
}

/// Complete active keybinding map across all modes, loaded from `$HOME/.config/lsi/keybinds.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keybinds {
    // Navigation & History
    pub move_down: Vec<KeyCombo>,
    pub move_up: Vec<KeyCombo>,
    pub page_down: Vec<KeyCombo>,
    pub page_up: Vec<KeyCombo>,
    pub jump_top: Vec<KeyCombo>,
    pub jump_bottom: Vec<KeyCombo>,
    pub go_parent: Vec<KeyCombo>,
    pub history_back: Vec<KeyCombo>,
    pub history_forward: Vec<KeyCombo>,
    pub jump_home: Vec<KeyCombo>,
    pub refresh: Vec<KeyCombo>,

    // Opening Files & Folders
    pub open: Vec<KeyCombo>,
    pub open_editor: Vec<KeyCombo>,
    pub open_system: Vec<KeyCombo>,

    // File Manipulation & Multi-Selection
    pub toggle_select: Vec<KeyCombo>,
    pub yank: Vec<KeyCombo>,
    pub cut: Vec<KeyCombo>,
    pub paste: Vec<KeyCombo>,
    pub trash: Vec<KeyCombo>,
    pub permanent_delete: Vec<KeyCombo>,
    pub confirm_delete: Vec<KeyCombo>,
    pub cancel_delete: Vec<KeyCombo>,
    pub create_item: Vec<KeyCombo>,
    pub create_confirm: Vec<KeyCombo>,
    pub create_cancel: Vec<KeyCombo>,
    pub rename_item: Vec<KeyCombo>,
    pub rename_clear: Vec<KeyCombo>,
    pub rename_confirm: Vec<KeyCombo>,
    pub rename_cancel: Vec<KeyCombo>,

    // View & Chrome Toggles
    pub toggle_hidden: Vec<KeyCombo>,
    pub toggle_permissions: Vec<KeyCombo>,
    pub toggle_ownership: Vec<KeyCombo>,
    pub toggle_time: Vec<KeyCombo>,
    pub toggle_size: Vec<KeyCombo>,
    pub toggle_icons: Vec<KeyCombo>,
    pub toggle_long: Vec<KeyCombo>,
    pub toggle_borders: Vec<KeyCombo>,
    pub toggle_footer: Vec<KeyCombo>,
    pub toggle_preview: Vec<KeyCombo>,

    // Sorting & Themes
    pub cycle_theme: Vec<KeyCombo>,
    pub cycle_sort: Vec<KeyCombo>,
    pub toggle_reverse_sort: Vec<KeyCombo>,
    pub toggle_dirs_first: Vec<KeyCombo>,

    // Search, Help & Exiting
    pub search: Vec<KeyCombo>,
    pub search_confirm: Vec<KeyCombo>,
    pub search_cancel: Vec<KeyCombo>,
    pub cancel: Vec<KeyCombo>,
    pub toggle_help: Vec<KeyCombo>,
    pub close_help: Vec<KeyCombo>,
    pub quit_cd: Vec<KeyCombo>,
    pub quit: Vec<KeyCombo>,
}

impl Default for Keybinds {
    fn default() -> Self {
        Self {
            move_down: parse_defaults(&["j", "Down"]),
            move_up: parse_defaults(&["k", "Up"]),
            page_down: parse_defaults(&["Ctrl+d", "PageDown"]),
            page_up: parse_defaults(&["Ctrl+u", "PageUp"]),
            jump_top: parse_defaults(&["g", "Home"]),
            jump_bottom: parse_defaults(&["G", "End"]),
            go_parent: parse_defaults(&["h", "Left", "Backspace"]),
            history_back: parse_defaults(&["-", "[", "b"]),
            history_forward: parse_defaults(&["+", "=", "]", "w"]),
            jump_home: parse_defaults(&["~"]),
            refresh: parse_defaults(&["F5", "Ctrl+r"]),

            open: parse_defaults(&["l", "Right", "Enter"]),
            open_editor: parse_defaults(&["e"]),
            open_system: parse_defaults(&["O"]),

            toggle_select: parse_defaults(&["Space"]),
            yank: parse_defaults(&["y"]),
            cut: parse_defaults(&["x"]),
            paste: parse_defaults(&["p"]),
            trash: parse_defaults(&["Delete"]),
            permanent_delete: parse_defaults(&["Shift+Delete"]),
            confirm_delete: parse_defaults(&["y", "Y", "Enter"]),
            cancel_delete: parse_defaults(&["n", "N", "Esc", "q"]),
            create_item: parse_defaults(&["a"]),
            create_confirm: parse_defaults(&["Enter"]),
            create_cancel: parse_defaults(&["Esc"]),
            rename_item: parse_defaults(&["r"]),
            rename_clear: parse_defaults(&["Ctrl+u"]),
            rename_confirm: parse_defaults(&["Enter"]),
            rename_cancel: parse_defaults(&["Esc"]),

            toggle_hidden: parse_defaults(&["."]),
            toggle_permissions: parse_defaults(&["P"]),
            toggle_ownership: parse_defaults(&["o", "u"]),
            toggle_time: parse_defaults(&["t"]),
            toggle_size: parse_defaults(&["S"]),
            toggle_icons: parse_defaults(&["i"]),
            toggle_long: parse_defaults(&["L"]),
            toggle_borders: parse_defaults(&["B"]),
            toggle_footer: parse_defaults(&["F"]),
            toggle_preview: parse_defaults(&["v", "Tab"]),

            cycle_theme: parse_defaults(&["T", "c"]),
            cycle_sort: parse_defaults(&["s"]),
            toggle_reverse_sort: parse_defaults(&["R"]),
            toggle_dirs_first: parse_defaults(&["d"]),

            search: parse_defaults(&["/"]),
            search_confirm: parse_defaults(&["Enter"]),
            search_cancel: parse_defaults(&["Esc"]),
            cancel: parse_defaults(&["Esc"]),
            toggle_help: parse_defaults(&["?"]),
            close_help: parse_defaults(&["Esc", "Enter"]),
            quit_cd: parse_defaults(&["q"]),
            quit: parse_defaults(&["Q", "Ctrl+c"]),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum KeySpecList {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct KeybindsConfig {
    move_down: Option<KeySpecList>,
    move_up: Option<KeySpecList>,
    page_down: Option<KeySpecList>,
    page_up: Option<KeySpecList>,
    jump_top: Option<KeySpecList>,
    jump_bottom: Option<KeySpecList>,
    go_parent: Option<KeySpecList>,
    history_back: Option<KeySpecList>,
    history_forward: Option<KeySpecList>,
    jump_home: Option<KeySpecList>,
    refresh: Option<KeySpecList>,

    open: Option<KeySpecList>,
    open_editor: Option<KeySpecList>,
    open_system: Option<KeySpecList>,

    toggle_select: Option<KeySpecList>,
    yank: Option<KeySpecList>,
    cut: Option<KeySpecList>,
    paste: Option<KeySpecList>,
    trash: Option<KeySpecList>,
    permanent_delete: Option<KeySpecList>,
    confirm_delete: Option<KeySpecList>,
    cancel_delete: Option<KeySpecList>,
    create_item: Option<KeySpecList>,
    create_confirm: Option<KeySpecList>,
    create_cancel: Option<KeySpecList>,
    rename_item: Option<KeySpecList>,
    rename_clear: Option<KeySpecList>,
    rename_confirm: Option<KeySpecList>,
    rename_cancel: Option<KeySpecList>,

    toggle_hidden: Option<KeySpecList>,
    toggle_permissions: Option<KeySpecList>,
    toggle_ownership: Option<KeySpecList>,
    toggle_time: Option<KeySpecList>,
    toggle_size: Option<KeySpecList>,
    toggle_icons: Option<KeySpecList>,
    toggle_long: Option<KeySpecList>,
    toggle_borders: Option<KeySpecList>,
    toggle_footer: Option<KeySpecList>,
    toggle_preview: Option<KeySpecList>,

    cycle_theme: Option<KeySpecList>,
    cycle_sort: Option<KeySpecList>,
    toggle_reverse_sort: Option<KeySpecList>,
    toggle_dirs_first: Option<KeySpecList>,

    search: Option<KeySpecList>,
    search_confirm: Option<KeySpecList>,
    search_cancel: Option<KeySpecList>,
    cancel: Option<KeySpecList>,
    toggle_help: Option<KeySpecList>,
    close_help: Option<KeySpecList>,
    quit_cd: Option<KeySpecList>,
    quit: Option<KeySpecList>,
}

impl Keybinds {
    /// Parses a `keybinds.toml` string, merging user overrides on top of defaults.
    pub fn from_toml_str(raw: &str) -> Result<Self> {
        let cfg: KeybindsConfig = toml::from_str(raw)?;
        let defaults = Self::default();

        let (mut move_down, o_move_down) = resolve_field(cfg.move_down, defaults.move_down)?;
        let (mut move_up, o_move_up) = resolve_field(cfg.move_up, defaults.move_up)?;
        let (mut page_down, o_page_down) = resolve_field(cfg.page_down, defaults.page_down)?;
        let (mut page_up, o_page_up) = resolve_field(cfg.page_up, defaults.page_up)?;
        let (mut jump_top, o_jump_top) = resolve_field(cfg.jump_top, defaults.jump_top)?;
        let (mut jump_bottom, o_jump_bottom) =
            resolve_field(cfg.jump_bottom, defaults.jump_bottom)?;
        let (mut go_parent, o_go_parent) = resolve_field(cfg.go_parent, defaults.go_parent)?;
        let (mut history_back, o_history_back) =
            resolve_field(cfg.history_back, defaults.history_back)?;
        let (mut history_forward, o_history_forward) =
            resolve_field(cfg.history_forward, defaults.history_forward)?;
        let (mut jump_home, o_jump_home) = resolve_field(cfg.jump_home, defaults.jump_home)?;
        let (mut refresh, o_refresh) = resolve_field(cfg.refresh, defaults.refresh)?;

        let (mut open, o_open) = resolve_field(cfg.open, defaults.open)?;
        let (mut open_editor, o_open_editor) =
            resolve_field(cfg.open_editor, defaults.open_editor)?;
        let (mut open_system, o_open_system) =
            resolve_field(cfg.open_system, defaults.open_system)?;

        let (mut toggle_select, o_toggle_select) =
            resolve_field(cfg.toggle_select, defaults.toggle_select)?;
        let (mut yank, o_yank) = resolve_field(cfg.yank, defaults.yank)?;
        let (mut cut, o_cut) = resolve_field(cfg.cut, defaults.cut)?;
        let (mut paste, o_paste) = resolve_field(cfg.paste, defaults.paste)?;
        let (mut trash, o_trash) = resolve_field(cfg.trash, defaults.trash)?;
        let (mut permanent_delete, o_permanent_delete) =
            resolve_field(cfg.permanent_delete, defaults.permanent_delete)?;
        let (mut confirm_delete, o_confirm_delete) =
            resolve_field(cfg.confirm_delete, defaults.confirm_delete)?;
        let (mut cancel_delete, o_cancel_delete) =
            resolve_field(cfg.cancel_delete, defaults.cancel_delete)?;
        let (mut create_item, o_create_item) =
            resolve_field(cfg.create_item, defaults.create_item)?;
        let (mut create_confirm, o_create_confirm) =
            resolve_field(cfg.create_confirm, defaults.create_confirm)?;
        let (mut create_cancel, o_create_cancel) =
            resolve_field(cfg.create_cancel, defaults.create_cancel)?;
        let (mut rename_item, o_rename_item) =
            resolve_field(cfg.rename_item, defaults.rename_item)?;
        let (mut rename_clear, o_rename_clear) =
            resolve_field(cfg.rename_clear, defaults.rename_clear)?;
        let (mut rename_confirm, o_rename_confirm) =
            resolve_field(cfg.rename_confirm, defaults.rename_confirm)?;
        let (mut rename_cancel, o_rename_cancel) =
            resolve_field(cfg.rename_cancel, defaults.rename_cancel)?;

        let (mut toggle_hidden, o_toggle_hidden) =
            resolve_field(cfg.toggle_hidden, defaults.toggle_hidden)?;
        let (mut toggle_permissions, o_toggle_permissions) =
            resolve_field(cfg.toggle_permissions, defaults.toggle_permissions)?;
        let (mut toggle_ownership, o_toggle_ownership) =
            resolve_field(cfg.toggle_ownership, defaults.toggle_ownership)?;
        let (mut toggle_time, o_toggle_time) =
            resolve_field(cfg.toggle_time, defaults.toggle_time)?;
        let (mut toggle_size, o_toggle_size) =
            resolve_field(cfg.toggle_size, defaults.toggle_size)?;
        let (mut toggle_icons, o_toggle_icons) =
            resolve_field(cfg.toggle_icons, defaults.toggle_icons)?;
        let (mut toggle_long, o_toggle_long) =
            resolve_field(cfg.toggle_long, defaults.toggle_long)?;
        let (mut toggle_borders, o_toggle_borders) =
            resolve_field(cfg.toggle_borders, defaults.toggle_borders)?;
        let (mut toggle_footer, o_toggle_footer) =
            resolve_field(cfg.toggle_footer, defaults.toggle_footer)?;
        let (mut toggle_preview, o_toggle_preview) =
            resolve_field(cfg.toggle_preview, defaults.toggle_preview)?;

        let (mut cycle_theme, o_cycle_theme) =
            resolve_field(cfg.cycle_theme, defaults.cycle_theme)?;
        let (mut cycle_sort, o_cycle_sort) = resolve_field(cfg.cycle_sort, defaults.cycle_sort)?;
        let (mut toggle_reverse_sort, o_toggle_reverse_sort) =
            resolve_field(cfg.toggle_reverse_sort, defaults.toggle_reverse_sort)?;
        let (mut toggle_dirs_first, o_toggle_dirs_first) =
            resolve_field(cfg.toggle_dirs_first, defaults.toggle_dirs_first)?;

        let (mut search, o_search) = resolve_field(cfg.search, defaults.search)?;
        let (mut search_confirm, o_search_confirm) =
            resolve_field(cfg.search_confirm, defaults.search_confirm)?;
        let (mut search_cancel, o_search_cancel) =
            resolve_field(cfg.search_cancel, defaults.search_cancel)?;
        let (mut cancel, o_cancel) = resolve_field(cfg.cancel, defaults.cancel)?;
        let (mut toggle_help, o_toggle_help) =
            resolve_field(cfg.toggle_help, defaults.toggle_help)?;
        let (close_help, _) = resolve_field(cfg.close_help, defaults.close_help)?;
        let (mut quit_cd, o_quit_cd) = resolve_field(cfg.quit_cd, defaults.quit_cd)?;
        let (mut quit, o_quit) = resolve_field(cfg.quit, defaults.quit)?;

        // Strip overridden keys from untouched defaults separately per input mode so
        // partial remaps take precedence without breaking sibling modes.
        strip_overridden_from_defaults(&mut [
            (&mut move_down, o_move_down),
            (&mut move_up, o_move_up),
            (&mut page_down, o_page_down),
            (&mut page_up, o_page_up),
            (&mut jump_top, o_jump_top),
            (&mut jump_bottom, o_jump_bottom),
            (&mut go_parent, o_go_parent),
            (&mut history_back, o_history_back),
            (&mut history_forward, o_history_forward),
            (&mut jump_home, o_jump_home),
            (&mut refresh, o_refresh),
            (&mut open, o_open),
            (&mut open_editor, o_open_editor),
            (&mut open_system, o_open_system),
            (&mut toggle_select, o_toggle_select),
            (&mut yank, o_yank),
            (&mut cut, o_cut),
            (&mut paste, o_paste),
            (&mut trash, o_trash),
            (&mut permanent_delete, o_permanent_delete),
            (&mut create_item, o_create_item),
            (&mut rename_item, o_rename_item),
            (&mut toggle_hidden, o_toggle_hidden),
            (&mut toggle_permissions, o_toggle_permissions),
            (&mut toggle_ownership, o_toggle_ownership),
            (&mut toggle_time, o_toggle_time),
            (&mut toggle_size, o_toggle_size),
            (&mut toggle_icons, o_toggle_icons),
            (&mut toggle_long, o_toggle_long),
            (&mut toggle_borders, o_toggle_borders),
            (&mut toggle_footer, o_toggle_footer),
            (&mut toggle_preview, o_toggle_preview),
            (&mut cycle_theme, o_cycle_theme),
            (&mut cycle_sort, o_cycle_sort),
            (&mut toggle_reverse_sort, o_toggle_reverse_sort),
            (&mut toggle_dirs_first, o_toggle_dirs_first),
            (&mut search, o_search),
            (&mut cancel, o_cancel),
            (&mut toggle_help, o_toggle_help),
            (&mut quit_cd, o_quit_cd),
            (&mut quit, o_quit),
        ]);

        strip_overridden_from_defaults(&mut [
            (&mut confirm_delete, o_confirm_delete),
            (&mut cancel_delete, o_cancel_delete),
        ]);

        strip_overridden_from_defaults(&mut [
            (&mut search_confirm, o_search_confirm),
            (&mut search_cancel, o_search_cancel),
        ]);

        strip_overridden_from_defaults(&mut [
            (&mut create_confirm, o_create_confirm),
            (&mut create_cancel, o_create_cancel),
        ]);

        strip_overridden_from_defaults(&mut [
            (&mut rename_confirm, o_rename_confirm),
            (&mut rename_cancel, o_rename_cancel),
            (&mut rename_clear, o_rename_clear),
        ]);

        Ok(Self {
            move_down,
            move_up,
            page_down,
            page_up,
            jump_top,
            jump_bottom,
            go_parent,
            history_back,
            history_forward,
            jump_home,
            refresh,
            open,
            open_editor,
            open_system,
            toggle_select,
            yank,
            cut,
            paste,
            trash,
            permanent_delete,
            confirm_delete,
            cancel_delete,
            create_item,
            create_confirm,
            create_cancel,
            rename_item,
            rename_clear,
            rename_confirm,
            rename_cancel,
            toggle_hidden,
            toggle_permissions,
            toggle_ownership,
            toggle_time,
            toggle_size,
            toggle_icons,
            toggle_long,
            toggle_borders,
            toggle_footer,
            toggle_preview,
            cycle_theme,
            cycle_sort,
            toggle_reverse_sort,
            toggle_dirs_first,
            search,
            search_confirm,
            search_cancel,
            cancel,
            toggle_help,
            close_help,
            quit_cd,
            quit,
        })
    }

    /// Returns `true` if `key` matches any binding in `bindings`.
    pub fn matches(bindings: &[KeyCombo], key: KeyEvent) -> bool {
        let event = KeyCombo::from_event(key);
        bindings.contains(&event)
    }

    /// Matches force-quit bindings from `self.quit` that are non-character keys or modified
    /// (`Ctrl`/`Alt`) so printable quit letters like `Q` do not hijack text-input prompts.
    pub fn matches_force_quit(&self, key: KeyEvent) -> bool {
        let event = KeyCombo::from_event(key);
        self.quit.iter().any(|combo| {
            *combo == event
                && (!matches!(combo.code, KeyCode::Char(_))
                    || combo
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT))
        })
    }

    /// Resolves a Normal-mode `KeyEvent` to the first matching `NormalAction`, if any.
    pub fn resolve_normal(&self, key: KeyEvent) -> Option<NormalAction> {
        let event = KeyCombo::from_event(key);
        self.normal_bindings()
            .into_iter()
            .find_map(|(bindings, action)| bindings.contains(&event).then_some(action))
    }

    /// Joins all key combos in `bindings` with `" / "`, or returns `"-"` if unbound.
    pub fn format_short(bindings: &[KeyCombo]) -> String {
        if bindings.is_empty() {
            return "-".to_string();
        }
        bindings
            .iter()
            .map(KeyCombo::display_short)
            .collect::<Vec<_>>()
            .join(" / ")
    }

    /// Formats the primary (first) key combo in `bindings`, or returns `"-"` if unbound.
    pub fn primary_short(bindings: &[KeyCombo]) -> String {
        bindings
            .first()
            .map(KeyCombo::display_short)
            .unwrap_or_else(|| "-".to_string())
    }

    fn normal_bindings(&self) -> [(&[KeyCombo], NormalAction); 41] {
        [
            (&self.quit_cd, NormalAction::QuitCd),
            (&self.quit, NormalAction::Quit),
            (&self.move_down, NormalAction::MoveDown),
            (&self.move_up, NormalAction::MoveUp),
            (&self.page_down, NormalAction::PageDown),
            (&self.page_up, NormalAction::PageUp),
            (&self.jump_top, NormalAction::JumpTop),
            (&self.jump_bottom, NormalAction::JumpBottom),
            (&self.go_parent, NormalAction::GoParent),
            (&self.open, NormalAction::Open),
            (&self.open_system, NormalAction::OpenSystem),
            (&self.open_editor, NormalAction::OpenEditor),
            (&self.history_back, NormalAction::HistoryBack),
            (&self.history_forward, NormalAction::HistoryForward),
            (&self.jump_home, NormalAction::JumpHome),
            (&self.refresh, NormalAction::Refresh),
            (&self.toggle_hidden, NormalAction::ToggleHidden),
            (&self.create_item, NormalAction::CreateItem),
            (&self.rename_item, NormalAction::RenameItem),
            (&self.toggle_permissions, NormalAction::TogglePermissions),
            (&self.toggle_ownership, NormalAction::ToggleOwnership),
            (&self.toggle_time, NormalAction::ToggleTime),
            (&self.toggle_size, NormalAction::ToggleSize),
            (&self.toggle_icons, NormalAction::ToggleIcons),
            (&self.toggle_borders, NormalAction::ToggleBorders),
            (&self.toggle_footer, NormalAction::ToggleFooter),
            (&self.toggle_long, NormalAction::ToggleLong),
            (&self.toggle_select, NormalAction::ToggleSelect),
            (&self.yank, NormalAction::Yank),
            (&self.cut, NormalAction::Cut),
            (&self.paste, NormalAction::Paste),
            (&self.trash, NormalAction::Trash),
            (&self.permanent_delete, NormalAction::PermanentDelete),
            (&self.cycle_theme, NormalAction::CycleTheme),
            (&self.cycle_sort, NormalAction::CycleSort),
            (&self.toggle_reverse_sort, NormalAction::ToggleReverseSort),
            (&self.toggle_dirs_first, NormalAction::ToggleDirsFirst),
            (&self.toggle_preview, NormalAction::TogglePreview),
            (&self.search, NormalAction::Search),
            (&self.cancel, NormalAction::Cancel),
            (&self.toggle_help, NormalAction::ToggleHelp),
        ]
    }
}

fn parse_defaults(specs: &[&str]) -> Vec<KeyCombo> {
    specs
        .iter()
        .map(|s| KeyCombo::parse(s).expect("built-in keybind default must be valid"))
        .collect()
}

fn resolve_field(spec: Option<KeySpecList>, default: Vec<KeyCombo>) -> Result<(Vec<KeyCombo>, bool)> {
    match spec {
        None => Ok((default, false)),
        Some(KeySpecList::Single(s)) => Ok((vec![KeyCombo::parse(&s)?], true)),
        Some(KeySpecList::Multiple(list)) => {
            let mut parsed = Vec::with_capacity(list.len());
            for s in &list {
                parsed.push(KeyCombo::parse(s)?);
            }
            Ok((parsed, true))
        }
    }
}

fn strip_overridden_from_defaults(group: &mut [(&mut Vec<KeyCombo>, bool)]) {
    let mut claimed = HashSet::new();
    for (keys, overridden) in group.iter() {
        if *overridden {
            for combo in keys.iter().copied() {
                claimed.insert(combo);
            }
        }
    }
    if claimed.is_empty() {
        return;
    }
    for (keys, overridden) in group.iter_mut() {
        if !*overridden {
            keys.retain(|combo| !claimed.contains(combo));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chars_named_keys_and_modifiers() {
        for single in ["+", "-", "?", "~", "/", ".", "[", "]", "=", "p", "P"] {
            let parsed = KeyCombo::parse(single).unwrap();
            let ch = single.chars().next().unwrap();
            assert_eq!(parsed.code, KeyCode::Char(ch));
            assert_eq!(parsed.modifiers, KeyModifiers::NONE);
        }

        assert_eq!(
            KeyCombo::parse(" ").unwrap(),
            KeyCombo {
                code: KeyCode::Char(' '),
                modifiers: KeyModifiers::NONE
            }
        );
        assert_eq!(
            KeyCombo::parse("Space").unwrap(),
            KeyCombo {
                code: KeyCode::Char(' '),
                modifiers: KeyModifiers::NONE
            }
        );
        assert_eq!(
            KeyCombo::parse("Delete").unwrap(),
            KeyCombo {
                code: KeyCode::Delete,
                modifiers: KeyModifiers::NONE
            }
        );
        assert_eq!(
            KeyCombo::parse("PageDown").unwrap(),
            KeyCombo {
                code: KeyCode::PageDown,
                modifiers: KeyModifiers::NONE
            }
        );
        assert_eq!(
            KeyCombo::parse("F5").unwrap(),
            KeyCombo {
                code: KeyCode::F(5),
                modifiers: KeyModifiers::NONE
            }
        );

        let ctrl_r = KeyCombo::parse("Ctrl+r").unwrap();
        assert_eq!(ctrl_r, KeyCombo::parse("Ctrl+R").unwrap());
        assert_eq!(ctrl_r.code, KeyCode::Char('r'));
        assert_eq!(ctrl_r.modifiers, KeyModifiers::CONTROL);
        assert_eq!(ctrl_r.display_short(), "Ctrl+r");

        let ctrl_shift_r = KeyCombo::parse("Ctrl+Shift+r").unwrap();
        assert_eq!(ctrl_shift_r.code, KeyCode::Char('R'));
        assert_eq!(ctrl_shift_r.modifiers, KeyModifiers::CONTROL);
        assert_eq!(ctrl_shift_r.display_short(), "Ctrl+Shift+R");

        assert_eq!(KeyCombo::parse("Shift+p").unwrap(), KeyCombo::parse("P").unwrap());

        let shift_del = KeyCombo::parse("Shift+Delete").unwrap();
        assert_eq!(shift_del.code, KeyCode::Delete);
        assert_eq!(shift_del.modifiers, KeyModifiers::SHIFT);
        assert_eq!(shift_del.display_short(), "Shift+Del");

        assert_eq!(
            KeyCombo::parse("Shift+Tab").unwrap(),
            KeyCombo::parse("BackTab").unwrap()
        );
        assert_eq!(KeyCombo::parse("BackTab").unwrap().display_short(), "Shift+Tab");

        let ctrl_plus = KeyCombo::parse("Ctrl++").unwrap();
        assert_eq!(ctrl_plus.code, KeyCode::Char('+'));
        assert_eq!(ctrl_plus.modifiers, KeyModifiers::CONTROL);

        let alt_x = KeyCombo::parse("Alt+x").unwrap();
        assert_eq!(alt_x.code, KeyCode::Char('x'));
        assert_eq!(alt_x.modifiers, KeyModifiers::ALT);
        assert_eq!(alt_x.display_short(), "Alt+x");
    }

    #[test]
    fn rejects_invalid_keys_and_unknown_toml_fields() {
        for bad in ["", "   ", "Ctrl+", "F0", "F25", "NotAKey"] {
            assert!(KeyCombo::parse(bad).is_err(), "expected error for {bad:?}");
        }

        assert!(Keybinds::from_toml_str("not_a_real_action = \"j\"").is_err());
        assert!(Keybinds::from_toml_str("move_down = \"NotAKey\"").is_err());
    }

    #[test]
    fn supports_single_array_unbind_and_mode_scoped_conflict_stripping() {
        let kb = Keybinds::from_toml_str(
            r#"
create_item = "c"
rename_item = ["F2", "Ctrl+Shift+r"]
toggle_hidden = []
"#,
        )
        .unwrap();

        assert_eq!(kb.create_item, parse_defaults(&["c"]));
        assert_eq!(kb.rename_item, parse_defaults(&["F2", "Ctrl+Shift+r"]));
        assert!(kb.toggle_hidden.is_empty());
        assert_eq!(kb.cycle_theme, parse_defaults(&["T"]));

        let kb_y = Keybinds::from_toml_str(r#"create_item = "y""#).unwrap();
        assert_eq!(kb_y.create_item, parse_defaults(&["y"]));
        assert!(kb_y.yank.is_empty());
        assert_eq!(kb_y.confirm_delete, parse_defaults(&["y", "Y", "Enter"]));

        assert_eq!(
            kb.resolve_normal(KeyEvent::from(KeyCode::Char('c'))),
            Some(NormalAction::CreateItem)
        );
        assert_eq!(
            kb.resolve_normal(KeyEvent::from(KeyCode::Char('.'))),
            None
        );
    }

    #[test]
    fn full_default_toml_round_trips_to_default() {
        let full_toml = r#"
move_down = ["j", "Down"]
move_up = ["k", "Up"]
page_down = ["Ctrl+d", "PageDown"]
page_up = ["Ctrl+u", "PageUp"]
jump_top = ["g", "Home"]
jump_bottom = ["G", "End"]
go_parent = ["h", "Left", "Backspace"]
history_back = ["-", "[", "b"]
history_forward = ["+", "=", "]", "w"]
jump_home = "~"
refresh = ["F5", "Ctrl+r"]

open = ["l", "Right", "Enter"]
open_editor = "e"
open_system = "O"

toggle_select = "Space"
yank = "y"
cut = "x"
paste = "p"
trash = "Delete"
permanent_delete = "Shift+Delete"
confirm_delete = ["y", "Y", "Enter"]
cancel_delete = ["n", "N", "Esc", "q"]
create_item = "a"
create_confirm = "Enter"
create_cancel = "Esc"
rename_item = "r"
rename_clear = "Ctrl+u"
rename_confirm = "Enter"
rename_cancel = "Esc"

toggle_hidden = "."
toggle_permissions = "P"
toggle_ownership = ["o", "u"]
toggle_time = "t"
toggle_size = "S"
toggle_icons = "i"
toggle_long = "L"
toggle_borders = "B"
toggle_footer = "F"
toggle_preview = ["v", "Tab"]

cycle_theme = ["T", "c"]
cycle_sort = "s"
toggle_reverse_sort = "R"
toggle_dirs_first = "d"

search = "/"
search_confirm = "Enter"
search_cancel = "Esc"
cancel = "Esc"
toggle_help = "?"
close_help = ["Esc", "Enter"]
quit_cd = "q"
quit = ["Q", "Ctrl+c"]
"#;

        let parsed = Keybinds::from_toml_str(full_toml).unwrap();
        assert_eq!(parsed, Keybinds::default());
    }
}
