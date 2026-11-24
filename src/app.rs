use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::TableState;

use crate::cli::{Args, SortField};
use crate::config::{load_named_theme, load_user_config, LoadedConfig, ThemeLoadOutcome};
use crate::fs_entry::{current_uid, read_directory_with_cache, FileEntry, IdentityCache};
use crate::fs_ops;
use crate::keybinds::{Keybinds, NormalAction};
#[cfg(feature = "image-preview")]
use crate::preview::is_image_extension;
use crate::preview::{
    create_image_picker, picker_uses_kitty, ImagePickerHandle, ImageProtocol, PreviewConfig,
    PreviewData, PreviewRequest, PreviewWorker,
};
use crate::theme::{Theme, ThemeName};

fn char_to_byte_idx(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(idx, _)| idx)
        .unwrap_or(s.len())
}

/// Current input mode for the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
    Help,
    ConfirmDelete,
    CreateItem,
    RenameItem,
}

/// Operation mode for yanked/cut clipboard items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardMode {
    Copy,
    Cut,
}

/// Clipboard storing paths marked for copy (`y`) or cut (`x`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clipboard {
    pub mode: ClipboardMode,
    pub paths: Vec<PathBuf>,
}

/// Action requested by `App` that the outer terminal loop should execute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppAction {
    None,
    Quit { change_dir: bool },
    OpenEditor(PathBuf),
}

/// Entry in the back/forward directory navigation stacks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryItem {
    pub dir: PathBuf,
    pub selected_name: Option<String>,
}

/// Application state for the interactive `lsi` explorer.
pub struct App {
    pub current_dir: PathBuf,
    pub all_entries: Vec<FileEntry>,
    pub visible_entries: Vec<FileEntry>,
    pub table_state: TableState,

    pub back_stack: Vec<HistoryItem>,
    pub forward_stack: Vec<HistoryItem>,
    pub dir_selection_memory: HashMap<PathBuf, String>,

    pub show_hidden: bool,
    pub show_permissions: bool,
    pub show_ownership: bool,
    pub show_time: bool,
    pub show_size: bool,
    pub show_icons: bool,
    pub show_preview: bool,
    pub show_borders: bool,
    pub show_footer: bool,
    pub max_rows: u16,

    pub theme_name: ThemeName,
    pub custom_theme: Option<Theme>,
    pub custom_theme_name: Option<String>,
    pub keybinds: Keybinds,
    pub sort_field: SortField,
    pub sort_reverse: bool,
    pub dirs_first: bool,

    pub input_mode: InputMode,
    pub search_query: String,
    pub create_input: String,
    pub rename_input: String,
    pub rename_cursor: usize,
    pub rename_target: Option<PathBuf>,
    pub status_message: Option<String>,

    pub selected_paths: HashSet<PathBuf>,
    pub clipboard: Option<Clipboard>,
    pub pending_delete: Vec<PathBuf>,
    trash_dir_override: Option<PathBuf>,

    pub current_uid: u32,
    pub identity_cache: IdentityCache,

    preview_path: Option<PathBuf>,
    pub preview_data: PreviewData,
    pub preview_config: PreviewConfig,
    pub preview_scroll: u16,
    preview_area: (u16, u16),
    preview_req_id: u64,
    pub preview_in_flight: bool,
    preview_started_at: Option<Instant>,
    preview_worker: Option<PreviewWorker>,
    image_picker: Option<Arc<ImagePickerHandle>>,
    image_picker_probed: bool,
    uses_kitty_graphics: bool,
    pending_graphics_clear: bool,
}

impl App {
    /// Initializes `App` from parsed CLI arguments and user config under `$HOME/.config/lsi`.
    pub fn new(args: Args) -> Result<Self> {
        Self::new_with_config(args, load_user_config())
    }

    /// Initializes `App` by merging `args` over `loaded` config and hard-coded defaults.
    pub fn new_with_config(args: Args, loaded: LoadedConfig) -> Result<Self> {
        let (start_dir, initial_select) = resolve_initial_path(args.path.as_deref())?;
        let cfg = &loaded.config;

        let show_hidden = args.all || cfg.show_hidden.unwrap_or(false);
        let long = args.long || cfg.long.unwrap_or(false);
        let show_permissions = long || args.permissions || cfg.show_permissions.unwrap_or(false);
        let show_ownership = long || args.ownership || cfg.show_ownership.unwrap_or(false);
        let show_time = long || args.time || cfg.show_time.unwrap_or(false);
        let show_size = if args.no_size {
            false
        } else if args.size || long {
            true
        } else {
            cfg.show_size.unwrap_or(true)
        };
        let show_icons = args.show_icons_override().or(cfg.show_icons).unwrap_or(true);
        let show_preview = args.preview || cfg.show_preview.unwrap_or(false);
        let show_borders = args
            .show_borders_override()
            .or(cfg.show_borders)
            .unwrap_or(true);
        let show_footer = args
            .show_footer_override()
            .or(cfg.show_footer)
            .unwrap_or(true);
        let dirs_first = args.dirs_first_override().or(cfg.dirs_first).unwrap_or(true);
        let sort_reverse = args.reverse || cfg.reverse.unwrap_or(false);
        let sort_field = args.sort.or(cfg.sort).unwrap_or(SortField::Name);
        let max_rows = args.max_rows.or(cfg.max_rows).unwrap_or(20).max(3);

        let mut custom_theme = loaded.custom_theme;
        let mut custom_theme_name = loaded.custom_theme_name;
        let mut status_message = loaded.warning.map(|w| format!("Config warning: {w}"));

        let theme_name = if let Some(cli_theme) = args.theme.as_deref() {
            let trimmed = cli_theme.trim();
            if let Some(builtin) = ThemeName::parse_builtin(trimmed) {
                builtin
            } else if custom_theme.is_some() && custom_theme_name.as_deref() == Some(trimmed) {
                ThemeName::Custom
            } else if let Some(ref themes_dir) = loaded.themes_dir {
                match load_named_theme(themes_dir, trimmed) {
                    ThemeLoadOutcome::Builtin(b) => b,
                    ThemeLoadOutcome::Custom { name, theme } => {
                        custom_theme = Some(theme);
                        custom_theme_name = Some(name);
                        ThemeName::Custom
                    }
                    ThemeLoadOutcome::NotFound { name } => {
                        status_message =
                            Some(format!("Theme '{name}' not found; falling back to eza-dark"));
                        ThemeName::EzaDark
                    }
                    ThemeLoadOutcome::Invalid { error } => {
                        status_message = Some(format!("Config warning: {error}"));
                        ThemeName::EzaDark
                    }
                }
            } else {
                status_message = Some(format!(
                    "Theme '{trimmed}' not found; falling back to eza-dark"
                ));
                ThemeName::EzaDark
            }
        } else if let Some(cfg_theme) = cfg.theme.as_deref() {
            if let Some(builtin) = ThemeName::parse_builtin(cfg_theme) {
                builtin
            } else if custom_theme.is_some() {
                ThemeName::Custom
            } else {
                if status_message.is_none() {
                    status_message = Some(format!(
                        "Theme '{}' not found; falling back to eza-dark",
                        cfg_theme.trim()
                    ));
                }
                ThemeName::EzaDark
            }
        } else {
            ThemeName::EzaDark
        };

        let preview_config = PreviewConfig {
            syntax_highlight: cfg.syntax_highlight.unwrap_or(true),
            image_preview: cfg.image_preview.unwrap_or(true),
            video_preview: cfg.video_preview.unwrap_or(true),
            image_protocol: cfg.image_protocol.unwrap_or(ImageProtocol::Auto),
            max_file_bytes: cfg
                .preview_max_file_mb
                .unwrap_or(20)
                .max(1)
                .saturating_mul(1024 * 1024),
        };
        let initial_picker = if preview_config.image_preview
            && preview_config.image_protocol != ImageProtocol::Off
        {
            create_image_picker(ImageProtocol::Halfblocks, false).map(Arc::new)
        } else {
            None
        };

        let mut app = Self {
            current_dir: start_dir,
            all_entries: Vec::new(),
            visible_entries: Vec::new(),
            table_state: TableState::default(),
            back_stack: Vec::new(),
            forward_stack: Vec::new(),
            dir_selection_memory: HashMap::new(),
            show_hidden,
            show_permissions,
            show_ownership,
            show_time,
            show_size,
            show_icons,
            show_preview,
            show_borders,
            show_footer,
            max_rows,
            theme_name,
            custom_theme,
            custom_theme_name,
            keybinds: loaded.keybinds,
            sort_field,
            sort_reverse,
            dirs_first,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            create_input: String::new(),
            rename_input: String::new(),
            rename_cursor: 0,
            rename_target: None,
            status_message,
            selected_paths: HashSet::new(),
            clipboard: None,
            pending_delete: Vec::new(),
            trash_dir_override: None,
            current_uid: current_uid(),
            identity_cache: IdentityCache::new(),
            preview_path: None,
            preview_data: PreviewData::Empty,
            preview_config,
            preview_scroll: 0,
            preview_area: (40, 18),
            preview_req_id: 0,
            preview_in_flight: false,
            preview_started_at: None,
            preview_worker: None,
            image_picker: initial_picker,
            image_picker_probed: false,
            uses_kitty_graphics: false,
            pending_graphics_clear: false,
        };

        app.load_directory(initial_select.as_deref())?;
        Ok(app)
    }

    /// Returns the active color palette.
    pub fn theme(&self) -> Theme {
        if self.theme_name == ThemeName::Custom {
            self.custom_theme
                .unwrap_or_else(|| ThemeName::EzaDark.palette())
        } else {
            self.theme_name.palette()
        }
    }

    /// Human-readable label for the active theme (using the custom filename stem when custom).
    pub fn active_theme_label(&self) -> &str {
        if self.theme_name == ThemeName::Custom {
            self.custom_theme_name.as_deref().unwrap_or("custom")
        } else {
            self.theme_name.as_str()
        }
    }

    /// Returns the currently highlighted `FileEntry`, if any.
    pub fn selected_entry(&self) -> Option<&FileEntry> {
        let idx = self.table_state.selected()?;
        self.visible_entries.get(idx)
    }

    /// Computes the desired inline viewport height based on visible entries, capped at `max_rows`.
    pub fn desired_inline_height(&self) -> u16 {
        if self.input_mode == InputMode::Help || self.show_preview {
            return self.max_rows;
        }
        let entry_rows = u16::try_from(self.visible_entries.len().max(1)).unwrap_or(u16::MAX);
        let compact = self.max_rows < 6;
        let border_rows: u16 = if self.show_borders && !compact { 2 } else { 0 };
        let footer_rows: u16 = if self.show_footer {
            if compact {
                1
            } else {
                2
            }
        } else if matches!(
            self.input_mode,
            InputMode::Search
                | InputMode::ConfirmDelete
                | InputMode::CreateItem
                | InputMode::RenameItem
        ) {
            1
        } else {
            0
        };
        let chrome_rows: u16 = 1_u16 + border_rows + footer_rows;
        chrome_rows
            .saturating_add(entry_rows)
            .min(self.max_rows)
            .max(chrome_rows.saturating_add(1).min(self.max_rows))
    }

    /// Reloads entries from `self.current_dir`, optionally selecting `preferred_name`.
    pub fn load_directory(&mut self, preferred_name: Option<&str>) -> Result<()> {
        let entries = read_directory_with_cache(&self.current_dir, &mut self.identity_cache)?;
        self.all_entries = entries;
        self.refresh_visible_entries(preferred_name);
        self.update_preview(true);
        Ok(())
    }

    /// Re-filters and re-sorts `visible_entries` from `all_entries`.
    pub fn refresh_visible_entries(&mut self, preferred_name: Option<&str>) {
        let current_selected = preferred_name
            .map(ToOwned::to_owned)
            .or_else(|| self.selected_entry().map(|e| e.name.clone()))
            .or_else(|| self.dir_selection_memory.get(&self.current_dir).cloned());

        let query = self.search_query.to_ascii_lowercase();
        let mut filtered: Vec<FileEntry> = self
            .all_entries
            .iter()
            .filter(|entry| self.show_hidden || !entry.is_hidden)
            .filter(|entry| query.is_empty() || entry.name.to_ascii_lowercase().contains(&query))
            .cloned()
            .collect();

        sort_entries(
            &mut filtered,
            self.sort_field,
            self.sort_reverse,
            self.dirs_first,
        );

        self.visible_entries = filtered;
        if self.visible_entries.is_empty() {
            self.table_state.select(None);
        } else if let Some(name) = current_selected {
            let idx = self
                .visible_entries
                .iter()
                .position(|e| e.name == name)
                .unwrap_or(0);
            self.table_state.select(Some(idx));
        } else {
            self.table_state.select(Some(0));
        }

        self.remember_current_selection();
        self.update_preview(false);
    }

    fn remember_current_selection(&mut self) {
        if let Some(name) = self.selected_entry().map(|e| e.name.clone()) {
            self.dir_selection_memory
                .insert(self.current_dir.clone(), name);
        }
    }

    fn current_history_item(&self) -> HistoryItem {
        HistoryItem {
            dir: self.current_dir.clone(),
            selected_name: self.selected_entry().map(|e| e.name.clone()),
        }
    }

    /// Navigates to `target_dir`, updating history stacks and restoring selection.
    fn navigate_to(
        &mut self,
        target_dir: PathBuf,
        preferred_name: Option<String>,
        record_history: bool,
    ) {
        let current = self.current_history_item();
        let prev_dir = self.current_dir.clone();
        let prev_query = std::mem::take(&mut self.search_query);

        self.current_dir = target_dir;
        match self.load_directory(preferred_name.as_deref()) {
            Ok(()) => {
                if record_history {
                    self.back_stack.push(current);
                    self.forward_stack.clear();
                }
                self.status_message = None;
            }
            Err(err) => {
                self.current_dir = prev_dir;
                self.search_query = prev_query;
                self.status_message = Some(format!("Cannot open directory: {err}"));
            }
        }
    }

    /// Navigates up to the parent directory, selecting the folder we exited.
    pub fn go_parent(&mut self) {
        let Some(parent) = self.current_dir.parent().map(Path::to_path_buf) else {
            self.status_message = Some("Already at filesystem root".to_string());
            return;
        };
        let child_name = self
            .current_dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned());
        self.navigate_to(parent, child_name, true);
    }

    /// Navigates back in history (`back_stack` -> `forward_stack`).
    pub fn go_back(&mut self) {
        let Some(prev) = self.back_stack.pop() else {
            self.status_message = Some("No previous directory in history".to_string());
            return;
        };
        let current = self.current_history_item();
        let fallback_dir = self.current_dir.clone();
        self.search_query.clear();
        self.current_dir = prev.dir;

        match self.load_directory(prev.selected_name.as_deref()) {
            Ok(()) => {
                self.forward_stack.push(current);
                self.status_message = None;
            }
            Err(err) => {
                self.current_dir = fallback_dir;
                self.status_message = Some(format!("History target unavailable: {err}"));
            }
        }
    }

    /// Navigates forward in history (`forward_stack` -> `back_stack`).
    pub fn go_forward(&mut self) {
        let Some(next) = self.forward_stack.pop() else {
            self.status_message = Some("No forward directory in history".to_string());
            return;
        };
        let current = self.current_history_item();
        let fallback_dir = self.current_dir.clone();
        self.search_query.clear();
        self.current_dir = next.dir;

        match self.load_directory(next.selected_name.as_deref()) {
            Ok(()) => {
                self.back_stack.push(current);
                self.status_message = None;
            }
            Err(err) => {
                self.current_dir = fallback_dir;
                self.status_message = Some(format!("History target unavailable: {err}"));
            }
        }
    }

    /// Opens the currently selected folder or file.
    ///
    /// Text files open in `$EDITOR` inside the current terminal unless `force_system_open` is true,
    /// while binary/media files use the system opener (`xdg-open` / `open::that_detached`).
    pub fn open_selected(&mut self, force_system_open: bool) -> AppAction {
        let Some(entry) = self.selected_entry().cloned() else {
            return AppAction::None;
        };

        if entry.is_dir {
            let remembered = self.dir_selection_memory.get(&entry.path).cloned();
            self.navigate_to(entry.path, remembered, true);
            return AppAction::None;
        }

        if entry.symlink_broken {
            self.status_message = Some(format!("Broken symlink: {}", entry.name));
            return AppAction::None;
        }

        if !force_system_open && is_likely_text_file(&entry.path) {
            return AppAction::OpenEditor(entry.path);
        }

        let has_gui_display = !cfg!(target_os = "linux")
            || std::env::var_os("DISPLAY").is_some()
            || std::env::var_os("WAYLAND_DISPLAY").is_some();

        if !has_gui_display {
            if is_likely_text_file(&entry.path) {
                return AppAction::OpenEditor(entry.path);
            }
            self.show_preview = true;
            self.update_preview(true);
            self.status_message = Some(format!(
                "No GUI display detected; previewing {}",
                entry.name
            ));
            return AppAction::None;
        }

        match open::that_detached(&entry.path) {
            Ok(()) => {
                self.status_message = Some(format!("Opened {}", entry.name));
                AppAction::None
            }
            Err(err) => {
                if is_likely_text_file(&entry.path) {
                    AppAction::OpenEditor(entry.path)
                } else {
                    self.show_preview = true;
                    self.update_preview(true);
                    self.status_message = Some(format!(
                        "Could not launch opener ({err}); previewing {}",
                        entry.name
                    ));
                    AppAction::None
                }
            }
        }
    }

    /// Opens the currently selected file unconditionally in `$EDITOR` (or navigates if directory).
    pub fn open_in_editor(&mut self) -> AppAction {
        let Some(entry) = self.selected_entry().cloned() else {
            return AppAction::None;
        };

        if entry.is_dir {
            let remembered = self.dir_selection_memory.get(&entry.path).cloned();
            self.navigate_to(entry.path, remembered, true);
            return AppAction::None;
        }

        if entry.symlink_broken {
            self.status_message = Some(format!("Broken symlink: {}", entry.name));
            return AppAction::None;
        }

        AppAction::OpenEditor(entry.path)
    }

    /// Moves selection by `delta` rows, clamping to valid bounds.
    pub fn move_selection(&mut self, delta: isize) {
        if self.visible_entries.is_empty() {
            self.table_state.select(None);
            return;
        }
        let len = self.visible_entries.len();
        let current = self.table_state.selected().unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, (len - 1) as isize) as usize;
        self.table_state.select(Some(next));
        self.remember_current_selection();
        self.update_preview(false);
    }

    /// Selects the first visible entry.
    pub fn select_first(&mut self) {
        if self.visible_entries.is_empty() {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(0));
            self.remember_current_selection();
            self.update_preview(false);
        }
    }

    /// Selects the last visible entry.
    pub fn select_last(&mut self) {
        if self.visible_entries.is_empty() {
            self.table_state.select(None);
        } else {
            self.table_state
                .select(Some(self.visible_entries.len() - 1));
            self.remember_current_selection();
            self.update_preview(false);
        }
    }

    /// Handles a keyboard event and returns any requested outer action.
    pub fn handle_key(&mut self, key: KeyEvent) -> AppAction {
        match self.input_mode {
            InputMode::Search => self.handle_search_key(key),
            InputMode::Help => self.handle_help_key(key),
            InputMode::ConfirmDelete => self.handle_confirm_delete_key(key),
            InputMode::CreateItem => self.handle_create_item_key(key),
            InputMode::RenameItem => self.handle_rename_item_key(key),
            InputMode::Normal => self.handle_normal_key(key),
        }
    }

    fn handle_confirm_delete_key(&mut self, key: KeyEvent) -> AppAction {
        if self.keybinds.matches_force_quit(key) {
            return AppAction::Quit { change_dir: false };
        }
        if Keybinds::matches(&self.keybinds.confirm_delete, key) {
            self.execute_permanent_delete();
            self.input_mode = InputMode::Normal;
            return AppAction::None;
        }
        if Keybinds::matches(&self.keybinds.cancel_delete, key) {
            self.pending_delete.clear();
            self.input_mode = InputMode::Normal;
            self.status_message = Some("Delete cancelled".to_string());
            return AppAction::None;
        }
        AppAction::None
    }

    fn handle_help_key(&mut self, key: KeyEvent) -> AppAction {
        if self.keybinds.matches_force_quit(key) {
            return AppAction::Quit { change_dir: false };
        }
        if Keybinds::matches(&self.keybinds.toggle_help, key)
            || Keybinds::matches(&self.keybinds.close_help, key)
        {
            self.input_mode = InputMode::Normal;
            return AppAction::None;
        }
        if Keybinds::matches(&self.keybinds.quit_cd, key) {
            return AppAction::Quit { change_dir: true };
        }
        if Keybinds::matches(&self.keybinds.quit, key) {
            return AppAction::Quit { change_dir: false };
        }
        AppAction::None
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> AppAction {
        if self.keybinds.matches_force_quit(key) {
            return AppAction::Quit { change_dir: false };
        }
        if Keybinds::matches(&self.keybinds.search_cancel, key) {
            self.search_query.clear();
            self.input_mode = InputMode::Normal;
            self.refresh_visible_entries(None);
            self.status_message = Some("Filter cleared".to_string());
            return AppAction::None;
        }
        if Keybinds::matches(&self.keybinds.search_confirm, key) {
            self.input_mode = InputMode::Normal;
            return AppAction::None;
        }
        match key.code {
            KeyCode::Backspace => {
                self.search_query.pop();
                self.refresh_visible_entries(None);
                AppAction::None
            }
            KeyCode::Char(ch)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.search_query.push(ch);
                self.refresh_visible_entries(None);
                AppAction::None
            }
            _ => AppAction::None,
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> AppAction {
        let Some(action) = self.keybinds.resolve_normal(key) else {
            return AppAction::None;
        };

        match action {
            NormalAction::QuitCd => AppAction::Quit { change_dir: true },
            NormalAction::Quit => AppAction::Quit { change_dir: false },

            // Vertical scrolling
            NormalAction::MoveDown => {
                self.move_selection(1);
                AppAction::None
            }
            NormalAction::MoveUp => {
                self.move_selection(-1);
                AppAction::None
            }
            NormalAction::PageDown => {
                self.move_selection(10);
                AppAction::None
            }
            NormalAction::PageUp => {
                self.move_selection(-10);
                AppAction::None
            }
            NormalAction::JumpTop => {
                self.select_first();
                AppAction::None
            }
            NormalAction::JumpBottom => {
                self.select_last();
                AppAction::None
            }

            // Tree navigation & opening
            NormalAction::GoParent => {
                self.go_parent();
                AppAction::None
            }
            NormalAction::Open => self.open_selected(false),
            NormalAction::OpenSystem => self.open_selected(true),
            NormalAction::OpenEditor => self.open_in_editor(),

            // Back / forward history
            NormalAction::HistoryBack => {
                self.go_back();
                AppAction::None
            }
            NormalAction::HistoryForward => {
                self.go_forward();
                AppAction::None
            }

            // Home jump & reload
            NormalAction::JumpHome => {
                if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
                    if home.is_dir() {
                        let remembered = self.dir_selection_memory.get(&home).cloned();
                        self.navigate_to(home, remembered, true);
                    }
                }
                AppAction::None
            }
            NormalAction::Refresh => {
                self.refresh_directory();
                AppAction::None
            }

            // Column & view toggles
            NormalAction::ToggleHidden => {
                self.show_hidden = !self.show_hidden;
                self.refresh_visible_entries(None);
                self.status_message = Some(format!(
                    "Hidden files: {}",
                    if self.show_hidden { "shown" } else { "hidden" }
                ));
                AppAction::None
            }
            NormalAction::CreateItem => {
                self.create_input.clear();
                self.input_mode = InputMode::CreateItem;
                AppAction::None
            }
            NormalAction::RenameItem => {
                self.start_rename();
                AppAction::None
            }
            NormalAction::TogglePermissions => {
                self.show_permissions = !self.show_permissions;
                AppAction::None
            }
            NormalAction::ToggleOwnership => {
                self.show_ownership = !self.show_ownership;
                AppAction::None
            }
            NormalAction::ToggleTime => {
                self.show_time = !self.show_time;
                AppAction::None
            }
            NormalAction::ToggleSize => {
                self.show_size = !self.show_size;
                AppAction::None
            }
            NormalAction::ToggleIcons => {
                self.show_icons = !self.show_icons;
                AppAction::None
            }
            NormalAction::ToggleBorders => {
                self.show_borders = !self.show_borders;
                self.status_message = Some(format!(
                    "Borders: {}",
                    if self.show_borders { "shown" } else { "hidden" }
                ));
                AppAction::None
            }
            NormalAction::ToggleFooter => {
                self.show_footer = !self.show_footer;
                self.status_message = Some(format!(
                    "Footer: {}",
                    if self.show_footer { "shown" } else { "hidden" }
                ));
                AppAction::None
            }
            NormalAction::ToggleLong => {
                let all_on = self.show_permissions
                    && self.show_ownership
                    && self.show_time
                    && self.show_size;
                let next = !all_on;
                self.show_permissions = next;
                self.show_ownership = next;
                self.show_time = next;
                self.show_size = next;
                AppAction::None
            }

            // Selection & file operations
            NormalAction::ToggleSelect => {
                self.toggle_select_current();
                AppAction::None
            }
            NormalAction::Yank => {
                self.yank_or_cut(ClipboardMode::Copy);
                AppAction::None
            }
            NormalAction::Cut => {
                self.yank_or_cut(ClipboardMode::Cut);
                AppAction::None
            }
            NormalAction::Paste => {
                self.paste_clipboard();
                AppAction::None
            }
            NormalAction::Trash => {
                self.trash_selected();
                AppAction::None
            }
            NormalAction::PermanentDelete => {
                self.request_permanent_delete();
                AppAction::None
            }

            // Themes & sorting
            NormalAction::CycleTheme => {
                self.theme_name = self.theme_name.next(self.custom_theme.is_some());
                self.status_message = Some(format!("Theme: {}", self.active_theme_label()));
                if self.show_preview {
                    self.update_preview(true);
                }
                AppAction::None
            }
            NormalAction::CycleSort => {
                self.sort_field = self.sort_field.next();
                self.refresh_visible_entries(None);
                self.status_message = Some(format!("Sort: {}", self.sort_field.as_str()));
                AppAction::None
            }
            NormalAction::ToggleReverseSort => {
                self.sort_reverse = !self.sort_reverse;
                self.refresh_visible_entries(None);
                self.status_message = Some(format!(
                    "Sort order: {}",
                    if self.sort_reverse {
                        "descending"
                    } else {
                        "ascending"
                    }
                ));
                AppAction::None
            }
            NormalAction::ToggleDirsFirst => {
                self.dirs_first = !self.dirs_first;
                self.refresh_visible_entries(None);
                self.status_message = Some(format!(
                    "Directories first: {}",
                    if self.dirs_first { "on" } else { "off" }
                ));
                AppAction::None
            }

            // Preview, search, help
            NormalAction::TogglePreview => {
                self.show_preview = !self.show_preview;
                if self.show_preview {
                    self.preview_scroll = 0;
                    self.update_preview(true);
                } else {
                    self.preview_path = None;
                    self.preview_data = PreviewData::Empty;
                    self.preview_scroll = 0;
                    self.preview_in_flight = false;
                    self.preview_started_at = None;
                    if self.uses_kitty_graphics {
                        self.pending_graphics_clear = true;
                        self.clear_preview_cache();
                    }
                }
                AppAction::None
            }
            NormalAction::PreviewScrollDown => {
                self.scroll_preview(3);
                AppAction::None
            }
            NormalAction::PreviewScrollUp => {
                self.scroll_preview(-3);
                AppAction::None
            }
            NormalAction::Search => {
                self.input_mode = InputMode::Search;
                AppAction::None
            }
            NormalAction::Cancel => {
                if !self.selected_paths.is_empty() {
                    self.selected_paths.clear();
                    self.status_message = Some("Selection cleared".to_string());
                    AppAction::None
                } else if !self.search_query.is_empty() {
                    self.search_query.clear();
                    self.refresh_visible_entries(None);
                    self.status_message = Some("Filter cleared".to_string());
                    AppAction::None
                } else {
                    AppAction::Quit { change_dir: false }
                }
            }
            NormalAction::ToggleHelp => {
                self.input_mode = InputMode::Help;
                AppAction::None
            }
        }
    }

    /// Returns the paths targeted by an operation: multi-selected items if any, else the current item.
    pub fn collect_target_paths(&self) -> Vec<PathBuf> {
        if !self.selected_paths.is_empty() {
            let mut ordered = Vec::with_capacity(self.selected_paths.len());
            let mut seen = HashSet::with_capacity(self.selected_paths.len());
            for entry in &self.visible_entries {
                if self.selected_paths.contains(&entry.path) && seen.insert(entry.path.clone()) {
                    ordered.push(entry.path.clone());
                }
            }
            let mut rest: Vec<PathBuf> = self
                .selected_paths
                .iter()
                .filter(|p| !seen.contains(*p))
                .cloned()
                .collect();
            rest.sort();
            ordered.extend(rest);
            return ordered;
        }
        self.selected_entry()
            .map(|e| vec![e.path.clone()])
            .unwrap_or_default()
    }

    fn toggle_select_current(&mut self) {
        let Some(entry) = self.selected_entry() else {
            return;
        };
        let path = entry.path.clone();
        if !self.selected_paths.remove(&path) {
            self.selected_paths.insert(path);
        }
        self.move_selection(1);
        let count = self.selected_paths.len();
        self.status_message = Some(if count == 0 {
            "Selection cleared".to_string()
        } else {
            format!("{count} item(s) selected")
        });
    }

    fn yank_or_cut(&mut self, mode: ClipboardMode) {
        let targets = self.collect_target_paths();
        if targets.is_empty() {
            self.status_message = Some("Nothing to select".to_string());
            return;
        }
        let count = targets.len();
        self.selected_paths.clear();
        let verb = match mode {
            ClipboardMode::Copy => "Yanked",
            ClipboardMode::Cut => "Cut",
        };
        self.clipboard = Some(Clipboard {
            mode,
            paths: targets,
        });
        self.status_message = Some(format!("{verb} {count} item(s)"));
    }

    fn paste_clipboard(&mut self) {
        let Some(clipboard) = self.clipboard.clone() else {
            self.status_message = Some("Clipboard is empty".to_string());
            return;
        };
        if clipboard.paths.is_empty() {
            self.clipboard = None;
            self.status_message = Some("Clipboard is empty".to_string());
            return;
        }

        let mut last_name: Option<String> = None;
        let mut succeeded = 0_usize;
        let mut skipped = 0_usize;
        let mut remaining_cut = Vec::new();
        let mut errors = Vec::new();

        for src in &clipboard.paths {
            match clipboard.mode {
                ClipboardMode::Copy => match fs_ops::copy_entry(src, &self.current_dir) {
                    Ok(dest) => {
                        succeeded += 1;
                        if let Some(name) = dest.file_name() {
                            last_name = Some(name.to_string_lossy().into_owned());
                        }
                    }
                    Err(err) => errors.push(err.to_string()),
                },
                ClipboardMode::Cut => match fs_ops::move_entry(src, &self.current_dir) {
                    Ok(Some(dest)) => {
                        succeeded += 1;
                        if let Some(name) = dest.file_name() {
                            last_name = Some(name.to_string_lossy().into_owned());
                        }
                    }
                    Ok(None) => {
                        skipped += 1;
                    }
                    Err(err) => {
                        remaining_cut.push(src.clone());
                        errors.push(err.to_string());
                    }
                },
            }
        }

        if clipboard.mode == ClipboardMode::Cut {
            if remaining_cut.is_empty() {
                self.clipboard = None;
            } else {
                self.clipboard = Some(Clipboard {
                    mode: ClipboardMode::Cut,
                    paths: remaining_cut,
                });
            }
        }

        if let Err(err) = self.load_directory(last_name.as_deref()) {
            self.status_message = Some(format!("Paste completed, refresh failed: {err}"));
            return;
        }

        let action_word = match clipboard.mode {
            ClipboardMode::Copy => "Copied",
            ClipboardMode::Cut => "Moved",
        };
        self.status_message = Some(format_batch_status(
            action_word,
            succeeded,
            skipped,
            &errors,
        ));
    }

    fn trash_selected(&mut self) {
        let targets = self.collect_target_paths();
        if targets.is_empty() {
            return;
        }

        let prev_idx = self.table_state.selected().unwrap_or(0);
        let prev_name = self.selected_entry().map(|e| e.name.clone());
        let mut trashed = Vec::new();
        let mut errors = Vec::new();

        for path in &targets {
            let res = match &self.trash_dir_override {
                Some(dir) => fs_ops::move_to_trash_in(path, dir),
                None => fs_ops::move_to_trash(path),
            };
            match res {
                Ok(_) => trashed.push(path.clone()),
                Err(err) => errors.push(err.to_string()),
            }
        }

        self.prune_paths_from_state(&trashed);
        self.reload_after_removal(prev_idx, prev_name.as_deref());

        let mut msg = format!("Moved {} item(s) to Trash", trashed.len());
        if let Some(first_err) = errors.first() {
            msg = format!("{msg} ({} error(s): {first_err})", errors.len());
        }
        self.status_message = Some(msg);
    }

    fn request_permanent_delete(&mut self) {
        let targets = self.collect_target_paths();
        if targets.is_empty() {
            return;
        }
        self.pending_delete = targets;
        self.input_mode = InputMode::ConfirmDelete;
    }

    fn execute_permanent_delete(&mut self) {
        let targets = std::mem::take(&mut self.pending_delete);
        if targets.is_empty() {
            return;
        }

        let prev_idx = self.table_state.selected().unwrap_or(0);
        let prev_name = self.selected_entry().map(|e| e.name.clone());
        let mut deleted = Vec::new();
        let mut errors = Vec::new();

        for path in &targets {
            match fs_ops::permanent_delete(path) {
                Ok(()) => deleted.push(path.clone()),
                Err(err) => errors.push(err.to_string()),
            }
        }

        self.prune_paths_from_state(&deleted);
        self.reload_after_removal(prev_idx, prev_name.as_deref());

        let mut msg = format!("Permanently deleted {} item(s)", deleted.len());
        if let Some(first_err) = errors.first() {
            msg = format!("{msg} ({} error(s): {first_err})", errors.len());
        }
        self.status_message = Some(msg);
    }

    fn handle_create_item_key(&mut self, key: KeyEvent) -> AppAction {
        if self.keybinds.matches_force_quit(key) {
            return AppAction::Quit { change_dir: false };
        }
        if Keybinds::matches(&self.keybinds.create_cancel, key) {
            self.create_input.clear();
            self.input_mode = InputMode::Normal;
            self.status_message = Some("Create cancelled".to_string());
            return AppAction::None;
        }
        if Keybinds::matches(&self.keybinds.create_confirm, key) {
            self.create_from_input();
            self.create_input.clear();
            self.input_mode = InputMode::Normal;
            return AppAction::None;
        }
        match key.code {
            KeyCode::Backspace => {
                self.create_input.pop();
                AppAction::None
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.create_input.push(c);
                AppAction::None
            }
            _ => AppAction::None,
        }
    }

    fn create_from_input(&mut self) {
        let trimmed = self.create_input.trim();
        if trimmed.is_empty() {
            self.status_message = Some("Create cancelled (empty name)".to_string());
            return;
        }
        let is_dir = trimmed.ends_with('/');
        match fs_ops::create_entry(&self.current_dir, trimmed) {
            Ok(created) => {
                let top_level = created
                    .strip_prefix(&self.current_dir)
                    .ok()
                    .and_then(|rel| rel.components().next())
                    .map(|c| c.as_os_str().to_string_lossy().into_owned());
                if let Err(err) = self.load_directory(top_level.as_deref()) {
                    self.status_message = Some(format!("Created, reload failed: {err}"));
                } else {
                    let kind = if is_dir { "folder" } else { "file" };
                    let rel_display = created
                        .strip_prefix(&self.current_dir)
                        .unwrap_or(&created)
                        .display();
                    self.status_message = Some(format!("Created {kind}: {rel_display}"));
                }
            }
            Err(err) => {
                self.status_message = Some(format!("Create failed: {err}"));
            }
        }
    }

    /// Returns `(before_cursor, after_cursor)` slices of `rename_input`.
    pub fn split_rename_input(&self) -> (&str, &str) {
        let byte_idx = char_to_byte_idx(&self.rename_input, self.rename_cursor);
        self.rename_input.split_at(byte_idx)
    }

    fn refresh_directory(&mut self) {
        if let Err(err) = self.load_directory(None) {
            self.status_message = Some(format!("Refresh failed: {err}"));
        } else {
            self.status_message = Some("Directory refreshed".to_string());
        }
    }

    fn start_rename(&mut self) {
        let Some((path, name)) = self
            .selected_entry()
            .map(|entry| (entry.path.clone(), entry.name.clone()))
        else {
            self.status_message = Some("Nothing to rename".to_string());
            return;
        };
        self.rename_target = Some(path);
        self.rename_input = name;
        self.rename_cursor = self.rename_input.chars().count();
        self.input_mode = InputMode::RenameItem;
    }

    fn handle_rename_item_key(&mut self, key: KeyEvent) -> AppAction {
        if self.keybinds.matches_force_quit(key) {
            return AppAction::Quit { change_dir: false };
        }
        if Keybinds::matches(&self.keybinds.rename_clear, key) {
            let byte_idx = char_to_byte_idx(&self.rename_input, self.rename_cursor);
            self.rename_input.drain(..byte_idx);
            self.rename_cursor = 0;
            return AppAction::None;
        }
        if Keybinds::matches(&self.keybinds.rename_cancel, key) {
            self.rename_input.clear();
            self.rename_cursor = 0;
            self.rename_target = None;
            self.input_mode = InputMode::Normal;
            self.status_message = Some("Rename cancelled".to_string());
            return AppAction::None;
        }
        if Keybinds::matches(&self.keybinds.rename_confirm, key) {
            self.rename_from_input();
            self.rename_input.clear();
            self.rename_cursor = 0;
            self.rename_target = None;
            self.input_mode = InputMode::Normal;
            return AppAction::None;
        }
        match key.code {
            KeyCode::Left => {
                self.rename_cursor = self.rename_cursor.saturating_sub(1);
                AppAction::None
            }
            KeyCode::Right => {
                let max_chars = self.rename_input.chars().count();
                if self.rename_cursor < max_chars {
                    self.rename_cursor += 1;
                }
                AppAction::None
            }
            KeyCode::Home => {
                self.rename_cursor = 0;
                AppAction::None
            }
            KeyCode::End => {
                self.rename_cursor = self.rename_input.chars().count();
                AppAction::None
            }
            KeyCode::Backspace => {
                if self.rename_cursor > 0 {
                    self.rename_cursor -= 1;
                    let byte_idx = char_to_byte_idx(&self.rename_input, self.rename_cursor);
                    self.rename_input.remove(byte_idx);
                }
                AppAction::None
            }
            KeyCode::Delete => {
                let max_chars = self.rename_input.chars().count();
                if self.rename_cursor < max_chars {
                    let byte_idx = char_to_byte_idx(&self.rename_input, self.rename_cursor);
                    self.rename_input.remove(byte_idx);
                }
                AppAction::None
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                let byte_idx = char_to_byte_idx(&self.rename_input, self.rename_cursor);
                self.rename_input.insert(byte_idx, c);
                self.rename_cursor += 1;
                AppAction::None
            }
            _ => AppAction::None,
        }
    }

    fn rename_from_input(&mut self) {
        let Some(old_path) = self.rename_target.take() else {
            return;
        };
        let trimmed = self.rename_input.trim().to_string();
        if trimmed.is_empty() {
            self.status_message = Some("Rename cancelled (empty name)".to_string());
            return;
        }
        if old_path.file_name().and_then(|n| n.to_str()) == Some(trimmed.as_str()) {
            self.status_message = Some("Rename unchanged".to_string());
            return;
        }

        match fs_ops::rename_entry(&old_path, &trimmed) {
            Ok(new_path) => {
                if self.selected_paths.remove(&old_path) {
                    self.selected_paths.insert(new_path.clone());
                }
                if let Some(ref mut clip) = self.clipboard {
                    for path in &mut clip.paths {
                        if *path == old_path {
                            *path = new_path.clone();
                        }
                    }
                }
                if let Some(mem) = self.dir_selection_memory.remove(&old_path) {
                    self.dir_selection_memory.insert(new_path, mem);
                }
                if let Err(err) = self.load_directory(Some(&trimmed)) {
                    self.status_message = Some(format!("Renamed, reload failed: {err}"));
                } else {
                    self.status_message = Some(format!("Renamed to: {trimmed}"));
                }
            }
            Err(err) => {
                self.status_message = Some(format!("Rename failed: {err}"));
            }
        }
    }

    fn prune_paths_from_state(&mut self, removed: &[PathBuf]) {
        if removed.is_empty() {
            return;
        }
        let removed_set: HashSet<&PathBuf> = removed.iter().collect();
        self.selected_paths.retain(|p| !removed_set.contains(p));
        if let Some(ref mut clip) = self.clipboard {
            clip.paths.retain(|p| !removed_set.contains(p));
            if clip.paths.is_empty() {
                self.clipboard = None;
            }
        }
    }

    fn reload_after_removal(&mut self, prev_idx: usize, prev_name: Option<&str>) {
        if let Err(err) = self.load_directory(None) {
            self.status_message = Some(format!("Reload failed: {err}"));
            return;
        }
        if self.visible_entries.is_empty() {
            return;
        }
        let still_present = prev_name
            .map(|name| self.visible_entries.iter().any(|e| e.name == name))
            .unwrap_or(false);
        if !still_present {
            let clamped = prev_idx.min(self.visible_entries.len() - 1);
            self.table_state.select(Some(clamped));
            self.remember_current_selection();
            self.update_preview(true);
        }
    }

    /// Writes `current_dir` to `cwd_file` when exiting with `change_dir == true`.
    pub fn write_cwd_file(&self, cwd_file: Option<&Path>, change_dir: bool) -> Result<()> {
        if change_dir {
            if let Some(path) = cwd_file {
                fs::write(path, self.current_dir.to_string_lossy().as_bytes())?;
            }
        }
        Ok(())
    }

    /// Lazily probes terminal image protocol capabilities once on a live TTY when preview opens.
    pub fn ensure_image_picker_ready(&mut self) {
        if !self.show_preview
            || self.image_picker_probed
            || !self.preview_config.image_preview
            || self.preview_config.image_protocol == ImageProtocol::Off
        {
            return;
        }
        self.image_picker_probed = true;
        let probed = create_image_picker(self.preview_config.image_protocol, true).map(Arc::new);
        self.uses_kitty_graphics = probed.as_deref().is_some_and(picker_uses_kitty);
        self.image_picker = probed.clone();
        if let Some(ref worker) = self.preview_worker {
            worker.update_picker(probed);
            self.update_preview(true);
        }
    }

    /// Returns whether the active terminal image protocol uses Kitty graphics overlays.
    pub const fn uses_kitty_graphics(&self) -> bool {
        self.uses_kitty_graphics
    }

    /// Returns `true` once when Kitty graphics overlays should be deleted from stdout.
    pub fn take_pending_graphics_clear(&mut self) -> bool {
        std::mem::take(&mut self.pending_graphics_clear)
    }

    /// Clears the background worker's preview cache (for example after clearing Kitty placements).
    pub fn clear_preview_cache(&mut self) {
        if let Some(ref worker) = self.preview_worker {
            worker.clear_cache();
        }
    }

    /// Clears cached preview graphics after terminal image storage has been purged (e.g., Kitty `d=A`).
    pub fn handle_graphics_cleared(&mut self) {
        self.clear_preview_cache();
        #[cfg(feature = "image-preview")]
        if matches!(self.preview_data, PreviewData::Image { .. }) {
            self.preview_data = PreviewData::Empty;
            if self.show_preview {
                self.update_preview(true);
            }
        }
    }

    /// Records the preview inner dimensions and refreshes image scaling when resized.
    pub fn set_preview_area(&mut self, width: u16, height: u16) {
        let next = (width.max(1), height.max(1));
        if self.preview_area == next {
            return;
        }
        self.preview_area = next;
        #[cfg(feature = "image-preview")]
        if self.show_preview
            && (matches!(self.preview_data, PreviewData::Image { .. })
                || (self.preview_in_flight
                    && self.selected_entry().is_some_and(|e| {
                        !e.is_dir
                            && (is_image_extension(&e.extension)
                                || (self.preview_config.video_preview
                                    && crate::video_preview::is_video_extension(&e.extension)))
                    })))
        {
            self.update_preview(true);
        }
    }

    /// Returns `true` when an async preview build has been pending long enough to show a notice.
    pub fn should_show_preview_loading(&self) -> bool {
        self.preview_in_flight
            && self
                .preview_started_at
                .is_some_and(|started| started.elapsed() >= Duration::from_millis(30))
    }

    /// Scrolls text or directory preview content vertically by `delta` rows.
    pub fn scroll_preview(&mut self, delta: i32) {
        if !self.show_preview {
            return;
        }
        let total_lines = match &self.preview_data {
            PreviewData::Text {
                lines, truncated, ..
            } => lines.len() + usize::from(*truncated),
            PreviewData::Directory {
                entries,
                total_count,
            } => entries.len() + usize::from(*total_count > entries.len()),
            PreviewData::Metadata { rows, note, .. } => {
                1 + rows.len() + if note.is_some() { 2 } else { 0 }
            }
            _ => 0,
        };
        let visible_rows = usize::from(self.preview_area.1.max(1));
        let max_scroll =
            u16::try_from(total_lines.saturating_sub(visible_rows)).unwrap_or(u16::MAX);
        if delta >= 0 {
            let step = u16::try_from(delta).unwrap_or(u16::MAX);
            self.preview_scroll = self.preview_scroll.saturating_add(step).min(max_scroll);
        } else {
            let step = u16::try_from(delta.saturating_abs()).unwrap_or(u16::MAX);
            self.preview_scroll = self.preview_scroll.saturating_sub(step);
        }
    }

    /// Dispatches an asynchronous preview update if `show_preview` is enabled and target changed or forced.
    pub fn update_preview(&mut self, force: bool) {
        if !self.show_preview {
            return;
        }
        let Some(selected) = self.selected_entry().cloned() else {
            self.preview_path = None;
            self.preview_data = PreviewData::Empty;
            self.preview_scroll = 0;
            self.preview_in_flight = false;
            self.preview_started_at = None;
            return;
        };

        let same_path = self.preview_path.as_ref() == Some(&selected.path);
        if !force && same_path {
            return;
        }
        if !same_path {
            self.preview_scroll = 0;
        }

        self.preview_path = Some(selected.path.clone());
        self.preview_req_id = self.preview_req_id.wrapping_add(1);
        self.preview_in_flight = true;
        self.preview_started_at = Some(Instant::now());

        if self.preview_worker.is_none() {
            self.preview_worker = Some(PreviewWorker::spawn(self.image_picker.clone()));
        }

        if let Some(ref worker) = self.preview_worker {
            worker.send_request(PreviewRequest {
                req_id: self.preview_req_id,
                entry: selected,
                pane_size: self.preview_area,
                show_hidden: self.show_hidden,
                is_light_theme: self.theme().is_light(),
                config: self.preview_config.clone(),
            });
        }
    }

    /// Drains completed responses from the background preview worker, returning `true` if updated.
    pub fn poll_preview_results(&mut self) -> bool {
        let Some(ref worker) = self.preview_worker else {
            return false;
        };
        let mut updated = false;
        while let Ok(resp) = worker.try_recv() {
            if resp.req_id != self.preview_req_id {
                continue;
            }
            if self.preview_path.as_ref() == Some(&resp.path) {
                self.preview_data = resp.data;
                self.preview_in_flight = false;
                self.preview_started_at = None;
                updated = true;
            }
        }
        updated
    }

    #[cfg(test)]
    pub fn wait_for_preview_for_test(&mut self, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        while self.preview_in_flight && Instant::now() < deadline {
            if self.poll_preview_results() {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

fn is_likely_text_file(path: &Path) -> bool {
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    let mut buf = [0_u8; 1024];
    let Ok(n) = file.read(&mut buf) else {
        return false;
    };
    !buf[..n].contains(&0)
}

fn resolve_initial_path(arg_path: Option<&Path>) -> Result<(PathBuf, Option<String>)> {
    let cwd = std::env::current_dir()?;
    let target = match arg_path {
        Some(p) if p.is_absolute() => p.to_path_buf(),
        Some(p) => cwd.join(p),
        None => cwd,
    };

    if !target.exists() && fs::symlink_metadata(&target).is_err() {
        bail!("Path does not exist: '{}'", target.display());
    }

    if target.is_dir() {
        let dir = fs::canonicalize(&target).unwrap_or(target);
        Ok((dir, None))
    } else {
        let file_name = target.file_name().map(|n| n.to_string_lossy().into_owned());
        let parent = target
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let dir = fs::canonicalize(&parent).unwrap_or(parent);
        Ok((dir, file_name))
    }
}

pub(crate) fn sort_entries(
    entries: &mut [FileEntry],
    field: SortField,
    reverse: bool,
    dirs_first: bool,
) {
    entries.sort_by(|a, b| {
        if dirs_first && a.is_dir != b.is_dir {
            return if a.is_dir {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }

        let cmp = match field {
            SortField::Name => compare_names(&a.name, &b.name),
            SortField::Size => a
                .size
                .cmp(&b.size)
                .then_with(|| compare_names(&a.name, &b.name)),
            SortField::Modified => a
                .modified
                .cmp(&b.modified)
                .then_with(|| compare_names(&a.name, &b.name)),
            SortField::Extension => a
                .extension
                .cmp(&b.extension)
                .then_with(|| compare_names(&a.name, &b.name)),
            SortField::Type => type_rank(a)
                .cmp(&type_rank(b))
                .then_with(|| a.extension.cmp(&b.extension))
                .then_with(|| compare_names(&a.name, &b.name)),
        };

        if reverse {
            cmp.reverse()
        } else {
            cmp
        }
    });
}

fn compare_names(a: &str, b: &str) -> Ordering {
    a.to_ascii_lowercase()
        .cmp(&b.to_ascii_lowercase())
        .then_with(|| a.cmp(b))
}

const fn type_rank(entry: &FileEntry) -> u8 {
    if entry.is_dir {
        0
    } else if entry.is_symlink {
        1
    } else if entry.is_executable {
        2
    } else {
        3
    }
}

fn format_batch_status(verb: &str, succeeded: usize, skipped: usize, errors: &[String]) -> String {
    let mut parts = Vec::new();
    if succeeded > 0 {
        parts.push(format!("{verb} {succeeded} item(s)"));
    }
    if skipped > 0 {
        parts.push(format!("{skipped} already here"));
    }
    if let Some(first_err) = errors.first() {
        parts.push(format!("{} error(s): {first_err}", errors.len()));
    }
    if parts.is_empty() {
        "Nothing changed".to_string()
    } else {
        parts.join("; ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn make_test_tree() -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "lsi_app_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("alpha_dir/nested")).unwrap();
        fs::create_dir_all(base.join("beta_dir")).unwrap();
        fs::create_dir_all(base.join(".hidden_dir")).unwrap();
        fs::write(base.join("alpha_dir/inner.txt"), "hello preview\nline 2\n").unwrap();
        fs::write(base.join("main.rs"), "fn main() {}\n").unwrap();
        fs::write(base.join(".env"), "SECRET=1\n").unwrap();
        base
    }

    #[test]
    fn app_navigation_history_toggles_and_quit() {
        let root = make_test_tree();
        let args = Args::try_parse_from(["lsi", root.to_str().unwrap()]).unwrap();
        let mut app = App::new_with_config(args, LoadedConfig::default()).unwrap();

        // Hidden files hidden by default
        assert!(!app.show_hidden);
        assert!(app.visible_entries.iter().all(|e| !e.is_hidden));

        // Toggle hidden files with '.'
        app.handle_key(KeyEvent::from(KeyCode::Char('.')));
        assert!(app.show_hidden);
        assert!(app.visible_entries.iter().any(|e| e.name == ".env"));

        // Toggle back off
        app.handle_key(KeyEvent::from(KeyCode::Char('.')));
        assert!(!app.show_hidden);

        // First item should be alpha_dir (dirs_first = true, sorted by name)
        assert_eq!(app.selected_entry().unwrap().name, "alpha_dir");

        // Enter alpha_dir
        app.handle_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(
            app.current_dir,
            fs::canonicalize(root.join("alpha_dir")).unwrap()
        );
        assert_eq!(app.back_stack.len(), 1);

        // Go back to parent with 'h'
        app.handle_key(KeyEvent::from(KeyCode::Char('h')));
        assert_eq!(app.current_dir, fs::canonicalize(&root).unwrap());
        assert_eq!(app.selected_entry().unwrap().name, "alpha_dir");

        // History back/forward ('-' goes back to alpha_dir because 'h' pushed alpha_dir onto back_stack)
        app.handle_key(KeyEvent::from(KeyCode::Char('-')));
        assert_eq!(
            app.current_dir,
            fs::canonicalize(root.join("alpha_dir")).unwrap()
        );
        assert_eq!(app.forward_stack.len(), 1);

        // '+' goes forward back to root
        app.handle_key(KeyEvent::from(KeyCode::Char('+')));
        assert_eq!(app.current_dir, fs::canonicalize(&root).unwrap());

        // Cycle themes with 'T'
        let initial_theme = app.theme_name;
        app.handle_key(KeyEvent::from(KeyCode::Char('T')));
        assert_eq!(app.theme_name, initial_theme.next(false));

        // Search filtering
        app.handle_key(KeyEvent::from(KeyCode::Char('/')));
        assert_eq!(app.input_mode, InputMode::Search);
        app.handle_key(KeyEvent::from(KeyCode::Char('m')));
        app.handle_key(KeyEvent::from(KeyCode::Char('a')));
        assert_eq!(app.visible_entries.len(), 1);
        assert_eq!(app.visible_entries[0].name, "main.rs");
        app.handle_key(KeyEvent::from(KeyCode::Esc));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.visible_entries.len(), 3);

        // Help modal and quit with 'q'
        app.handle_key(KeyEvent::from(KeyCode::Char('?')));
        assert_eq!(app.input_mode, InputMode::Help);
        assert_eq!(
            app.handle_key(KeyEvent::from(KeyCode::Char('q'))),
            AppAction::Quit { change_dir: true }
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn initial_file_argument_selects_file_in_parent_dir() {
        let root = make_test_tree();
        let file_path = root.join("main.rs");
        let args = Args::try_parse_from(["lsi", file_path.to_str().unwrap()]).unwrap();
        let app = App::new_with_config(args, LoadedConfig::default()).unwrap();

        assert_eq!(app.current_dir, fs::canonicalize(&root).unwrap());
        assert_eq!(app.selected_entry().unwrap().name, "main.rs");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn desired_inline_height_scales_and_caps_at_max_rows() {
        let root = make_test_tree();
        // root has 3 non-hidden entries: alpha_dir, beta_dir, main.rs
        let args = Args::try_parse_from(["lsi", "-n", "9", root.to_str().unwrap()]).unwrap();
        let mut app = App::new_with_config(args, LoadedConfig::default()).unwrap();
        assert_eq!(app.visible_entries.len(), 3);
        assert_eq!(app.desired_inline_height(), 8);

        // Hiding borders drops 2 rows, hiding footer drops 2 more rows
        app.handle_key(KeyEvent::from(KeyCode::Char('B')));
        assert!(!app.show_borders);
        assert_eq!(app.desired_inline_height(), 6);
        app.handle_key(KeyEvent::from(KeyCode::Char('F')));
        assert!(!app.show_footer);
        assert_eq!(app.desired_inline_height(), 4);
        // Searching when footer is hidden adds 1 row for search prompt
        app.handle_key(KeyEvent::from(KeyCode::Char('/')));
        assert_eq!(app.desired_inline_height(), 5);
        app.handle_key(KeyEvent::from(KeyCode::Esc));
        app.handle_key(KeyEvent::from(KeyCode::Char('B')));
        app.handle_key(KeyEvent::from(KeyCode::Char('F')));
        assert_eq!(app.desired_inline_height(), 8);

        // Showing hidden adds .hidden_dir and .env -> 5 entries => 5 + 5 = 10, capped at 9
        app.handle_key(KeyEvent::from(KeyCode::Char('.')));
        assert_eq!(app.visible_entries.len(), 5);
        assert_eq!(app.desired_inline_height(), 9);

        // Help or preview uses full max_rows
        app.handle_key(KeyEvent::from(KeyCode::Char('.')));
        assert_eq!(app.desired_inline_height(), 8);
        app.handle_key(KeyEvent::from(KeyCode::Char('v')));
        assert_eq!(app.desired_inline_height(), 9);
        app.handle_key(KeyEvent::from(KeyCode::Char('v')));
        assert_eq!(app.desired_inline_height(), 8);
        app.handle_key(KeyEvent::from(KeyCode::Char('?')));
        assert_eq!(app.desired_inline_height(), 9);

        // Small max_rows (< 6) uses compact chrome (2)
        let small_args = Args::try_parse_from(["lsi", "-n", "5", root.to_str().unwrap()]).unwrap();
        let small_app = App::new_with_config(small_args, LoadedConfig::default()).unwrap();
        assert_eq!(small_app.desired_inline_height(), 5);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn enter_and_l_open_text_files_in_editor() {
        let root = make_test_tree();
        let file_path = fs::canonicalize(root.join("main.rs")).unwrap();
        let args = Args::try_parse_from(["lsi", file_path.to_str().unwrap()]).unwrap();
        let mut app = App::new_with_config(args, LoadedConfig::default()).unwrap();

        assert_eq!(app.selected_entry().unwrap().name, "main.rs");
        assert_eq!(
            app.handle_key(KeyEvent::from(KeyCode::Enter)),
            AppAction::OpenEditor(file_path.clone())
        );
        assert_eq!(
            app.handle_key(KeyEvent::from(KeyCode::Char('l'))),
            AppAction::OpenEditor(file_path)
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn config_precedence_and_custom_theme_default_and_cycle() {
        use crate::config::load_from_dir;
        use ratatui::style::Color;

        let root = make_test_tree();
        let cfg_dir = make_test_tree();
        fs::create_dir_all(cfg_dir.join("themes")).unwrap();
        fs::write(
            cfg_dir.join("config.toml"),
            r#"
show_borders = false
show_footer = false
show_hidden = true
show_permissions = true
show_size = false
max_rows = 15
theme = "catppuccin-mocha-mauve"
"#,
        )
        .unwrap();
        fs::write(
            cfg_dir.join("themes/catppuccin-mocha-mauve.toml"),
            "accent = \"#123456\"\n",
        )
        .unwrap();
        fs::write(
            cfg_dir.join("themes/catppuccin.toml"),
            "accent = \"#abcdef\"\n",
        )
        .unwrap();
        fs::write(
            cfg_dir.join("themes/cli-custom.toml"),
            "accent = \"#654321\"\n",
        )
        .unwrap();

        let loaded = load_from_dir(&cfg_dir);

        // Without CLI overrides: config wins and loads themes/catppuccin-mocha-mauve.toml
        let args = Args::try_parse_from(["lsi", root.to_str().unwrap()]).unwrap();
        let mut app = App::new_with_config(args, loaded.clone()).unwrap();
        assert!(!app.show_borders);
        assert!(!app.show_footer);
        assert!(app.show_hidden);
        assert!(app.show_permissions);
        assert!(!app.show_size);
        assert_eq!(app.max_rows, 15);
        assert_eq!(app.theme_name, ThemeName::Custom);
        assert_eq!(app.active_theme_label(), "catppuccin-mocha-mauve");
        assert_eq!(app.theme().accent, Color::Rgb(0x12, 0x34, 0x56));
        assert_eq!(app.theme().code_fg, ThemeName::EzaDark.palette().code_fg);

        // Cycling moves through built-in themes and back to Custom
        app.handle_key(KeyEvent::from(KeyCode::Char('T')));
        assert_eq!(app.theme_name, ThemeName::EzaDark);
        for _ in 0..ThemeName::BUILTIN.len() {
            app.handle_key(KeyEvent::from(KeyCode::Char('T')));
        }
        assert_eq!(app.theme_name, ThemeName::Custom);
        assert_eq!(app.active_theme_label(), "catppuccin-mocha-mauve");

        // Explicit CLI flags override config.toml (built-in and named custom from themes/)
        let cli_override = Args::try_parse_from([
            "lsi",
            "--borders",
            "--footer",
            "--size",
            "--theme",
            "nord",
            "-n",
            "30",
            root.to_str().unwrap(),
        ])
        .unwrap();
        let app_cli = App::new_with_config(cli_override, loaded.clone()).unwrap();
        assert!(app_cli.show_borders);
        assert!(app_cli.show_footer);
        assert!(app_cli.show_size);
        assert_eq!(app_cli.theme_name, ThemeName::Nord);
        assert_eq!(app_cli.max_rows, 30);

        let cli_custom = Args::try_parse_from([
            "lsi",
            "--theme",
            "cli-custom",
            root.to_str().unwrap(),
        ])
        .unwrap();
        let app_cli_custom = App::new_with_config(cli_custom, loaded).unwrap();
        assert_eq!(app_cli_custom.theme_name, ThemeName::Custom);
        assert_eq!(app_cli_custom.active_theme_label(), "cli-custom");
        assert_eq!(app_cli_custom.theme().accent, Color::Rgb(0x65, 0x43, 0x21));

        // Built-in theme names ignore matching files in themes/ (catppuccin.toml is ignored)
        fs::write(cfg_dir.join("config.toml"), "theme = \"catppuccin\"\n").unwrap();
        let cat_loaded = load_from_dir(&cfg_dir);
        let app_cat = App::new_with_config(
            Args::try_parse_from(["lsi", root.to_str().unwrap()]).unwrap(),
            cat_loaded,
        )
        .unwrap();
        assert_eq!(app_cat.theme_name, ThemeName::Catppuccin);
        assert_eq!(app_cat.theme().accent, ThemeName::Catppuccin.palette().accent);

        // Missing custom theme falls back to eza-dark
        fs::write(cfg_dir.join("config.toml"), "theme = \"nonexistent\"\n").unwrap();
        let missing_loaded = load_from_dir(&cfg_dir);
        let app_missing = App::new_with_config(
            Args::try_parse_from(["lsi", root.to_str().unwrap()]).unwrap(),
            missing_loaded,
        )
        .unwrap();
        assert_eq!(app_missing.theme_name, ThemeName::EzaDark);

        let _ = fs::remove_dir_all(&cfg_dir);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn quit_change_dir_and_cwd_file_behavior() {
        let root = make_test_tree();
        let cwd_file = root.join("lsi-cwd.txt");
        let args = Args::try_parse_from(["lsi", root.to_str().unwrap()]).unwrap();
        let mut app = App::new_with_config(args, LoadedConfig::default()).unwrap();

        // Enter alpha_dir so current_dir differs from root
        app.handle_key(KeyEvent::from(KeyCode::Enter));
        let alpha_dir = fs::canonicalize(root.join("alpha_dir")).unwrap();
        assert_eq!(app.current_dir, alpha_dir);

        // 'Q' quits without changing dir and does not write cwd_file
        assert_eq!(
            app.handle_key(KeyEvent::from(KeyCode::Char('Q'))),
            AppAction::Quit { change_dir: false }
        );
        app.write_cwd_file(Some(&cwd_file), false).unwrap();
        assert!(!cwd_file.exists());

        // With locked search filter in Normal mode, first Esc clears filter, second Esc quits (no cd)
        app.handle_key(KeyEvent::from(KeyCode::Char('/')));
        app.handle_key(KeyEvent::from(KeyCode::Char('i')));
        app.handle_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.search_query, "i");
        assert_eq!(
            app.handle_key(KeyEvent::from(KeyCode::Esc)),
            AppAction::None
        );
        assert!(app.search_query.is_empty());
        assert_eq!(
            app.handle_key(KeyEvent::from(KeyCode::Esc)),
            AppAction::Quit { change_dir: false }
        );

        // 'q' quits with change_dir: true and writes current_dir to cwd_file
        assert_eq!(
            app.handle_key(KeyEvent::from(KeyCode::Char('q'))),
            AppAction::Quit { change_dir: true }
        );
        app.write_cwd_file(Some(&cwd_file), true).unwrap();
        let written = fs::read_to_string(&cwd_file).unwrap();
        assert_eq!(PathBuf::from(written), alpha_dir);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn space_multiselect_esc_priority_and_permissions_shift_p() {
        let root = make_test_tree();
        let args = Args::try_parse_from(["lsi", root.to_str().unwrap()]).unwrap();
        let mut app = App::new_with_config(args, LoadedConfig::default()).unwrap();

        // 'P' toggles permissions column; 'p' is paste (leaves permissions unchanged)
        assert!(!app.show_permissions);
        app.handle_key(KeyEvent::from(KeyCode::Char('p')));
        assert!(!app.show_permissions);
        app.handle_key(KeyEvent::from(KeyCode::Char('P')));
        assert!(app.show_permissions);

        // Activate a search filter first, then multi-select with Space
        app.handle_key(KeyEvent::from(KeyCode::Char('/')));
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        app.handle_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(app.visible_entries.len(), 2); // alpha_dir, beta_dir

        // Space selects alpha_dir and advances to beta_dir
        assert_eq!(app.selected_entry().unwrap().name, "alpha_dir");
        app.handle_key(KeyEvent::from(KeyCode::Char(' ')));
        assert_eq!(app.selected_paths.len(), 1);
        assert_eq!(app.selected_entry().unwrap().name, "beta_dir");

        // Space selects beta_dir
        app.handle_key(KeyEvent::from(KeyCode::Char(' ')));
        assert_eq!(app.selected_paths.len(), 2);

        // Esc 1: clears multi-selection first (keeps filter active)
        assert_eq!(
            app.handle_key(KeyEvent::from(KeyCode::Esc)),
            AppAction::None
        );
        assert!(app.selected_paths.is_empty());
        assert_eq!(app.search_query, "d");

        // Esc 2: clears search filter
        assert_eq!(
            app.handle_key(KeyEvent::from(KeyCode::Esc)),
            AppAction::None
        );
        assert!(app.search_query.is_empty());

        // Esc 3: quits without cd
        assert_eq!(
            app.handle_key(KeyEvent::from(KeyCode::Esc)),
            AppAction::Quit { change_dir: false }
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn copy_cut_paste_trash_and_permanent_delete() {
        let root = make_test_tree();
        let trash_dir = root.join(".test_trash");
        let args = Args::try_parse_from(["lsi", root.to_str().unwrap()]).unwrap();
        let mut app = App::new_with_config(args, LoadedConfig::default()).unwrap();
        app.trash_dir_override = Some(trash_dir.clone());

        // Select main.rs (3rd visible entry: alpha_dir, beta_dir, main.rs)
        app.move_selection(2);
        assert_eq!(app.selected_entry().unwrap().name, "main.rs");

        // Yank ('y') and paste ('p') in same directory creates main_copy.rs
        app.handle_key(KeyEvent::from(KeyCode::Char('y')));
        assert_eq!(app.clipboard.as_ref().unwrap().mode, ClipboardMode::Copy);
        app.handle_key(KeyEvent::from(KeyCode::Char('p')));
        assert!(root.join("main_copy.rs").is_file());
        assert!(root.join("main.rs").is_file());
        // Clipboard remains intact after copy-paste
        assert!(app.clipboard.is_some());

        // Select main_copy.rs, cut ('x'), navigate into beta_dir, and paste ('p')
        assert_eq!(app.selected_entry().unwrap().name, "main_copy.rs");
        app.handle_key(KeyEvent::from(KeyCode::Char('x')));
        assert_eq!(app.clipboard.as_ref().unwrap().mode, ClipboardMode::Cut);

        app.select_first();
        app.move_selection(1); // beta_dir
        assert_eq!(app.selected_entry().unwrap().name, "beta_dir");
        app.handle_key(KeyEvent::from(KeyCode::Enter));
        app.handle_key(KeyEvent::from(KeyCode::Char('p')));
        assert!(root.join("beta_dir/main_copy.rs").is_file());
        assert!(!root.join("main_copy.rs").exists());
        assert!(app.clipboard.is_none());

        // Delete moves beta_dir/main_copy.rs to trash_dir
        app.handle_key(KeyEvent::from(KeyCode::Delete));
        assert!(!root.join("beta_dir/main_copy.rs").exists());
        assert!(trash_dir.join("files/main_copy.rs").is_file());
        assert!(trash_dir.join("info/main_copy.rs.trashinfo").is_file());

        // Go back up to root, add an extra file, test Shift+Delete cancel ('n') then confirm ('y')
        app.handle_key(KeyEvent::from(KeyCode::Char('h')));
        fs::write(root.join("doomed.txt"), "bye").unwrap();
        app.load_directory(Some("doomed.txt")).unwrap();
        assert_eq!(app.selected_entry().unwrap().name, "doomed.txt");

        app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::SHIFT));
        assert_eq!(app.input_mode, InputMode::ConfirmDelete);
        assert_eq!(app.desired_inline_height(), app.desired_inline_height());
        app.handle_key(KeyEvent::from(KeyCode::Char('n')));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(root.join("doomed.txt").is_file());

        app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::SHIFT));
        assert_eq!(app.input_mode, InputMode::ConfirmDelete);
        app.handle_key(KeyEvent::from(KeyCode::Char('y')));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(!root.join("doomed.txt").exists());
        assert!(!trash_dir.join("files/doomed.txt").exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn create_new_file_and_folder_via_a_key_and_toggle_hidden_via_dot() {
        let root = make_test_tree();
        let args = Args::try_parse_from(["lsi", root.to_str().unwrap()]).unwrap();
        let mut app = App::new_with_config(args, LoadedConfig::default()).unwrap();
        assert!(!app.show_hidden);

        // '.' still toggles hidden files
        app.handle_key(KeyEvent::from(KeyCode::Char('.')));
        assert!(app.show_hidden);
        app.handle_key(KeyEvent::from(KeyCode::Char('.')));
        assert!(!app.show_hidden);

        // 'a' opens CreateItem prompt without toggling hidden files
        app.handle_key(KeyEvent::from(KeyCode::Char('a')));
        assert_eq!(app.input_mode, InputMode::CreateItem);
        assert!(!app.show_hidden);

        // Esc cancels creation and leaves filesystem untouched
        app.handle_key(KeyEvent::from(KeyCode::Char('x')));
        app.handle_key(KeyEvent::from(KeyCode::Backspace));
        for ch in "temp.txt".chars() {
            app.handle_key(KeyEvent::from(KeyCode::Char(ch)));
        }
        assert_eq!(app.create_input, "temp.txt");
        app.handle_key(KeyEvent::from(KeyCode::Esc));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.create_input.is_empty());
        assert!(!root.join("temp.txt").exists());

        // Create a new file via 'a' + name + Enter
        app.handle_key(KeyEvent::from(KeyCode::Char('a')));
        for ch in "notes.md".chars() {
            app.handle_key(KeyEvent::from(KeyCode::Char(ch)));
        }
        app.handle_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(root.join("notes.md").is_file());
        assert_eq!(app.selected_entry().unwrap().name, "notes.md");

        // Create a new folder via 'a' + trailing '/' + Enter
        app.handle_key(KeyEvent::from(KeyCode::Char('a')));
        for ch in "newdir/".chars() {
            app.handle_key(KeyEvent::from(KeyCode::Char(ch)));
        }
        app.handle_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(root.join("newdir").is_dir());
        assert_eq!(app.selected_entry().unwrap().name, "newdir");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rename_file_extension_folder_cursor_editing_and_reassigned_keys() {
        let root = make_test_tree();
        let args = Args::try_parse_from(["lsi", root.to_str().unwrap()]).unwrap();
        let mut app = App::new_with_config(args, LoadedConfig::default()).unwrap();

        // Highlight main.rs (visible: alpha_dir, beta_dir, main.rs) and rename its extension to .md
        app.move_selection(2);
        assert_eq!(app.selected_entry().unwrap().name, "main.rs");
        app.handle_key(KeyEvent::from(KeyCode::Char('r')));
        assert_eq!(app.input_mode, InputMode::RenameItem);
        assert_eq!(app.rename_input, "main.rs");
        assert_eq!(app.rename_cursor, "main.rs".chars().count());

        app.handle_key(KeyEvent::from(KeyCode::Backspace));
        app.handle_key(KeyEvent::from(KeyCode::Backspace));
        app.handle_key(KeyEvent::from(KeyCode::Char('m')));
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        app.handle_key(KeyEvent::from(KeyCode::Enter));

        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(!root.join("main.rs").exists());
        assert!(root.join("main.md").is_file());
        let renamed = app.selected_entry().unwrap();
        assert_eq!(renamed.name, "main.md");
        assert_eq!(renamed.extension, "md");

        // Mark alpha_dir in both selected_paths and clipboard, then rename via Left/Home/Delete/Ctrl+u
        app.select_first();
        assert_eq!(app.selected_entry().unwrap().name, "alpha_dir");
        let old_alpha_path = app.selected_entry().unwrap().path.clone();
        app.selected_paths.insert(old_alpha_path.clone());
        app.clipboard = Some(Clipboard {
            mode: ClipboardMode::Copy,
            paths: vec![old_alpha_path.clone()],
        });

        app.handle_key(KeyEvent::from(KeyCode::Char('r')));
        assert_eq!(app.rename_input, "alpha_dir");
        // Move left across "_dir" (4 chars) and delete to test Left/Home/Delete/Ctrl+u/UTF-8
        for _ in 0..4 {
            app.handle_key(KeyEvent::from(KeyCode::Left));
        }
        assert_eq!(app.split_rename_input(), ("alpha", "_dir"));
        app.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(app.split_rename_input(), ("", "_dir"));
        for ch in "café".chars() {
            app.handle_key(KeyEvent::from(KeyCode::Char(ch)));
        }
        app.handle_key(KeyEvent::from(KeyCode::Home));
        app.handle_key(KeyEvent::from(KeyCode::Delete)); // remove 'c' -> "afé_dir"
        app.handle_key(KeyEvent::from(KeyCode::Char('c'))); // restore 'c' -> "café_dir"
        app.handle_key(KeyEvent::from(KeyCode::End));
        app.handle_key(KeyEvent::from(KeyCode::Enter));

        assert!(root.join("café_dir").is_dir());
        assert!(!root.join("alpha_dir").exists());
        let expected_new_path = old_alpha_path.parent().unwrap().join("café_dir");
        assert_eq!(app.selected_entry().unwrap().name, "café_dir");
        assert!(app.selected_paths.contains(&expected_new_path));
        assert!(!app.selected_paths.contains(&old_alpha_path));
        assert_eq!(
            app.clipboard.as_ref().unwrap().paths,
            vec![expected_new_path]
        );

        // Esc cancels rename without modifying disk
        app.handle_key(KeyEvent::from(KeyCode::Char('r')));
        app.handle_key(KeyEvent::from(KeyCode::Backspace));
        app.handle_key(KeyEvent::from(KeyCode::Esc));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(root.join("café_dir").is_dir());

        // 'R' toggles reverse sort order
        assert!(!app.sort_reverse);
        app.handle_key(KeyEvent::from(KeyCode::Char('R')));
        assert!(app.sort_reverse);
        app.handle_key(KeyEvent::from(KeyCode::Char('R')));
        assert!(!app.sort_reverse);

        // Ctrl+r and F5 refresh directory contents
        fs::write(root.join("external_ctrl_r.txt"), "1").unwrap();
        app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
        assert!(app
            .visible_entries
            .iter()
            .any(|e| e.name == "external_ctrl_r.txt"));

        fs::write(root.join("external_f5.txt"), "2").unwrap();
        app.handle_key(KeyEvent::from(KeyCode::F(5)));
        assert!(app
            .visible_entries
            .iter()
            .any(|e| e.name == "external_f5.txt"));

        // Pressing 'r' in an empty directory stays in Normal mode
        let empty_dir = root.join("empty_sub");
        fs::create_dir(&empty_dir).unwrap();
        app.load_directory(Some("empty_sub")).unwrap();
        app.handle_key(KeyEvent::from(KeyCode::Enter));
        assert!(app.visible_entries.is_empty());
        app.handle_key(KeyEvent::from(KeyCode::Char('r')));
        assert_eq!(app.input_mode, InputMode::Normal);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn custom_keybinds_override_and_unbind_actions() {
        let root = make_test_tree();
        let keybinds = Keybinds::from_toml_str(
            r#"
create_item = "c"
rename_item = "F2"
move_down = "n"
toggle_hidden = []
confirm_delete = "d"
rename_clear = "Ctrl+k"
"#,
        )
        .unwrap();
        let loaded = LoadedConfig {
            keybinds,
            ..LoadedConfig::default()
        };
        let args = Args::try_parse_from(["lsi", root.to_str().unwrap()]).unwrap();
        let mut app = App::new_with_config(args, loaded).unwrap();

        // 'n' moves down (remapped move_down)
        assert_eq!(app.selected_entry().unwrap().name, "alpha_dir");
        app.handle_key(KeyEvent::from(KeyCode::Char('n')));
        assert_eq!(app.selected_entry().unwrap().name, "beta_dir");

        // '.' is unbound and does nothing
        assert!(!app.show_hidden);
        app.handle_key(KeyEvent::from(KeyCode::Char('.')));
        assert!(!app.show_hidden);

        // 'c' now opens CreateItem instead of cycling theme (while 'T' still cycles theme)
        let theme_before = app.theme_name;
        app.handle_key(KeyEvent::from(KeyCode::Char('T')));
        assert_ne!(app.theme_name, theme_before);
        app.handle_key(KeyEvent::from(KeyCode::Char('c')));
        assert_eq!(app.input_mode, InputMode::CreateItem);
        app.handle_key(KeyEvent::from(KeyCode::Esc));
        assert_eq!(app.input_mode, InputMode::Normal);

        // 'F2' opens RenameItem and Ctrl+k clears the prefilled input
        app.handle_key(KeyEvent::from(KeyCode::F(2)));
        assert_eq!(app.input_mode, InputMode::RenameItem);
        assert_eq!(app.rename_input, "beta_dir");
        app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL));
        assert!(app.rename_input.is_empty());
        app.handle_key(KeyEvent::from(KeyCode::Esc));
        assert_eq!(app.input_mode, InputMode::Normal);

        // 'd' confirms permanent delete when remapped
        app.move_selection(1); // main.rs
        assert_eq!(app.selected_entry().unwrap().name, "main.rs");
        app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::SHIFT));
        assert_eq!(app.input_mode, InputMode::ConfirmDelete);
        app.handle_key(KeyEvent::from(KeyCode::Char('d')));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(!root.join("main.rs").exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn async_preview_worker_loads_and_scrolls_and_clears_stale_state() {
        let root = make_test_tree();
        let mut long_code = String::new();
        for idx in 1..=60 {
            long_code.push_str(&format!("let v_{idx} = {idx};\n"));
        }
        fs::write(root.join("long.rs"), long_code).unwrap();

        let args = Args::try_parse_from(["lsi", "--preview", root.to_str().unwrap()]).unwrap();
        let mut app = App::new_with_config(args, LoadedConfig::default()).unwrap();
        app.set_preview_area(40, 10);

        // Initial selection is alpha_dir
        app.wait_for_preview_for_test(Duration::from_secs(2));
        assert!(!app.preview_in_flight);
        assert!(matches!(app.preview_data, PreviewData::Directory { .. }));

        // Move to long.rs (visible: alpha_dir, beta_dir, long.rs, main.rs)
        app.move_selection(2);
        assert_eq!(app.selected_entry().unwrap().name, "long.rs");
        app.wait_for_preview_for_test(Duration::from_secs(2));
        match &app.preview_data {
            PreviewData::Text {
                lines, truncated, ..
            } => {
                assert!(!truncated);
                assert_eq!(lines.len(), 60);
            }
            other => panic!("expected Text preview for long.rs, got {other:?}"),
        }

        // Scroll preview down and up via 'J' / 'K'
        app.handle_key(KeyEvent::from(KeyCode::Char('J')));
        assert_eq!(app.preview_scroll, 3);
        app.handle_key(KeyEvent::from(KeyCode::Char('K')));
        assert_eq!(app.preview_scroll, 0);

        // Scrolling down clamps cleanly and resets when moving selection
        for _ in 0..30 {
            app.handle_key(KeyEvent::from(KeyCode::Char('J')));
        }
        assert_eq!(app.preview_scroll, 50);
        app.move_selection(1); // main.rs
        assert_eq!(app.preview_scroll, 0);
        app.wait_for_preview_for_test(Duration::from_secs(2));

        // Toggling preview off clears preview_path and preview_data immediately
        app.handle_key(KeyEvent::from(KeyCode::Char('v')));
        assert!(!app.show_preview);
        assert!(app.preview_path.is_none());
        assert!(matches!(app.preview_data, PreviewData::Empty));
        assert_eq!(app.preview_scroll, 0);

        #[cfg(feature = "image-preview")]
        {
            let img: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
                image::ImageBuffer::from_pixel(64, 64, image::Rgba([20, 180, 90, 255]));
            img.save(root.join("photo.png")).unwrap();
            app.load_directory(Some("photo.png")).unwrap();
            assert_eq!(app.selected_entry().unwrap().name, "photo.png");

            app.set_preview_area(40, 18);
            app.handle_key(KeyEvent::from(KeyCode::Char('v')));
            assert!(app.show_preview);
            assert!(app.preview_in_flight);
            let req_before = app.preview_req_id;

            // Resizing the pane while an image preview is still in-flight re-dispatches with the new dimensions.
            app.set_preview_area(20, 10);
            assert_eq!(app.preview_req_id, req_before + 1);
            app.wait_for_preview_for_test(Duration::from_secs(2));
            match &app.preview_data {
                PreviewData::Image {
                    protocol,
                    width,
                    height,
                    ..
                } => {
                    assert_eq!((*width, *height), (64, 64));
                    assert!(protocol
                        .needs_placeholder(ratatui::layout::Rect::new(0, 0, 20, 10))
                        .is_none());
                }
                other => panic!("expected Image preview for photo.png, got {other:?}"),
            }

            app.handle_graphics_cleared();
            assert!(matches!(app.preview_data, PreviewData::Empty));
            assert!(app.preview_in_flight);
            app.wait_for_preview_for_test(Duration::from_secs(2));
            assert!(matches!(app.preview_data, PreviewData::Image { .. }));
        }

        let _ = fs::remove_dir_all(&root);
    }
}
