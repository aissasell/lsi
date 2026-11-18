use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::TableState;

use crate::cli::{Args, SortField};
use crate::config::{load_user_config, LoadedConfig};
use crate::fs_entry::{current_uid, read_directory_with_cache, FileEntry, IdentityCache};
use crate::theme::{Theme, ThemeName};

const PREVIEW_MAX_LINES: usize = 100;
const PREVIEW_MAX_BYTES: usize = 32 * 1024;
const PREVIEW_DIR_LIMIT: usize = 100;

/// Current input mode for the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
    Help,
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

/// Cached content for the side-by-side preview pane.
#[derive(Debug, Clone)]
pub enum PreviewData {
    Empty,
    Directory {
        entries: Vec<FileEntry>,
        total_count: usize,
    },
    Text {
        lines: Vec<String>,
        truncated: bool,
    },
    Binary {
        size: u64,
    },
    Error(String),
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
    pub sort_field: SortField,
    pub sort_reverse: bool,
    pub dirs_first: bool,

    pub input_mode: InputMode,
    pub search_query: String,
    pub status_message: Option<String>,

    pub current_uid: u32,
    pub identity_cache: IdentityCache,

    preview_path: Option<PathBuf>,
    pub preview_data: PreviewData,
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

        let mut theme_name = args.theme.or(cfg.theme).unwrap_or(
            if loaded.custom_theme.is_some() {
                ThemeName::Custom
            } else {
                ThemeName::EzaDark
            },
        );
        let mut status_message = loaded.warning.map(|w| format!("Config warning: {w}"));
        if theme_name == ThemeName::Custom && loaded.custom_theme.is_none() {
            theme_name = ThemeName::EzaDark;
            if status_message.is_none() {
                status_message = Some("No theme.toml found; falling back to eza-dark".to_string());
            }
        }

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
            custom_theme: loaded.custom_theme,
            sort_field,
            sort_reverse,
            dirs_first,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            status_message,
            current_uid: current_uid(),
            identity_cache: IdentityCache::new(),
            preview_path: None,
            preview_data: PreviewData::Empty,
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
        } else if self.input_mode == InputMode::Search {
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
            InputMode::Normal => self.handle_normal_key(key),
        }
    }

    fn handle_help_key(&mut self, key: KeyEvent) -> AppAction {
        match key.code {
            KeyCode::Char('q') => AppAction::Quit { change_dir: true },
            KeyCode::Char('Q') => AppAction::Quit { change_dir: false },
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                AppAction::Quit { change_dir: false }
            }
            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Enter => {
                self.input_mode = InputMode::Normal;
                AppAction::None
            }
            _ => AppAction::None,
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> AppAction {
        match key.code {
            KeyCode::Esc => {
                self.search_query.clear();
                self.input_mode = InputMode::Normal;
                self.refresh_visible_entries(None);
                self.status_message = Some("Filter cleared".to_string());
                AppAction::None
            }
            KeyCode::Enter => {
                self.input_mode = InputMode::Normal;
                AppAction::None
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.refresh_visible_entries(None);
                AppAction::None
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                AppAction::Quit { change_dir: false }
            }
            KeyCode::Char(ch) => {
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT)
                {
                    self.search_query.push(ch);
                    self.refresh_visible_entries(None);
                }
                AppAction::None
            }
            _ => AppAction::None,
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> AppAction {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char('c') => AppAction::Quit { change_dir: false },
                KeyCode::Char('d') => {
                    self.move_selection(10);
                    AppAction::None
                }
                KeyCode::Char('u') => {
                    self.move_selection(-10);
                    AppAction::None
                }
                _ => AppAction::None,
            };
        }

        match key.code {
            KeyCode::Char('q') => AppAction::Quit { change_dir: true },
            KeyCode::Char('Q') => AppAction::Quit { change_dir: false },

            // Vertical scrolling
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_selection(1);
                AppAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_selection(-1);
                AppAction::None
            }
            KeyCode::PageDown => {
                self.move_selection(10);
                AppAction::None
            }
            KeyCode::PageUp => {
                self.move_selection(-10);
                AppAction::None
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.select_first();
                AppAction::None
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.select_last();
                AppAction::None
            }

            // Tree navigation & opening
            KeyCode::Char('h') | KeyCode::Left | KeyCode::Backspace => {
                self.go_parent();
                AppAction::None
            }
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => self.open_selected(false),
            KeyCode::Char('O') => self.open_selected(true),
            KeyCode::Char('e') => self.open_in_editor(),

            // Back / forward history
            KeyCode::Char('-') | KeyCode::Char('[') | KeyCode::Char('b') => {
                self.go_back();
                AppAction::None
            }
            KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Char(']') | KeyCode::Char('w') => {
                self.go_forward();
                AppAction::None
            }

            // Home jump & reload
            KeyCode::Char('~') => {
                if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
                    if home.is_dir() {
                        let remembered = self.dir_selection_memory.get(&home).cloned();
                        self.navigate_to(home, remembered, true);
                    }
                }
                AppAction::None
            }
            KeyCode::Char('R') | KeyCode::F(5) => {
                if let Err(err) = self.load_directory(None) {
                    self.status_message = Some(format!("Refresh failed: {err}"));
                } else {
                    self.status_message = Some("Directory refreshed".to_string());
                }
                AppAction::None
            }

            // Column & view toggles
            KeyCode::Char('.') | KeyCode::Char('a') => {
                self.show_hidden = !self.show_hidden;
                self.refresh_visible_entries(None);
                self.status_message = Some(format!(
                    "Hidden files: {}",
                    if self.show_hidden { "shown" } else { "hidden" }
                ));
                AppAction::None
            }
            KeyCode::Char('p') => {
                self.show_permissions = !self.show_permissions;
                AppAction::None
            }
            KeyCode::Char('o') | KeyCode::Char('u') => {
                self.show_ownership = !self.show_ownership;
                AppAction::None
            }
            KeyCode::Char('t') => {
                self.show_time = !self.show_time;
                AppAction::None
            }
            KeyCode::Char('S') => {
                self.show_size = !self.show_size;
                AppAction::None
            }
            KeyCode::Char('i') => {
                self.show_icons = !self.show_icons;
                AppAction::None
            }
            KeyCode::Char('B') => {
                self.show_borders = !self.show_borders;
                self.status_message = Some(format!(
                    "Borders: {}",
                    if self.show_borders { "shown" } else { "hidden" }
                ));
                AppAction::None
            }
            KeyCode::Char('F') => {
                self.show_footer = !self.show_footer;
                self.status_message = Some(format!(
                    "Footer: {}",
                    if self.show_footer { "shown" } else { "hidden" }
                ));
                AppAction::None
            }
            KeyCode::Char('L') => {
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

            // Themes & sorting
            KeyCode::Char('T') | KeyCode::Char('c') => {
                self.theme_name = self.theme_name.next(self.custom_theme.is_some());
                self.status_message = Some(format!("Theme: {}", self.theme_name.as_str()));
                AppAction::None
            }
            KeyCode::Char('s') => {
                self.sort_field = self.sort_field.next();
                self.refresh_visible_entries(None);
                self.status_message = Some(format!("Sort: {}", self.sort_field.as_str()));
                AppAction::None
            }
            KeyCode::Char('r') => {
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
            KeyCode::Char('d') => {
                self.dirs_first = !self.dirs_first;
                self.refresh_visible_entries(None);
                self.status_message = Some(format!(
                    "Directories first: {}",
                    if self.dirs_first { "on" } else { "off" }
                ));
                AppAction::None
            }

            // Preview, search, help
            KeyCode::Char('v') | KeyCode::Tab => {
                self.show_preview = !self.show_preview;
                self.update_preview(true);
                AppAction::None
            }
            KeyCode::Char('/') => {
                self.input_mode = InputMode::Search;
                AppAction::None
            }
            KeyCode::Esc => {
                if !self.search_query.is_empty() {
                    self.search_query.clear();
                    self.refresh_visible_entries(None);
                    self.status_message = Some("Filter cleared".to_string());
                    AppAction::None
                } else {
                    AppAction::Quit { change_dir: false }
                }
            }
            KeyCode::Char('?') => {
                self.input_mode = InputMode::Help;
                AppAction::None
            }
            _ => AppAction::None,
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

    /// Recomputes cached preview data if `show_preview` is enabled and target changed or forced.
    pub fn update_preview(&mut self, force: bool) {
        if !self.show_preview {
            return;
        }
        let Some(selected) = self.selected_entry().cloned() else {
            self.preview_path = None;
            self.preview_data = PreviewData::Empty;
            return;
        };

        if !force && self.preview_path.as_ref() == Some(&selected.path) {
            return;
        }

        self.preview_path = Some(selected.path.clone());
        self.preview_data = build_preview(&selected, &mut self.identity_cache, self.show_hidden);
    }
}

fn build_preview(entry: &FileEntry, cache: &mut IdentityCache, show_hidden: bool) -> PreviewData {
    if entry.symlink_broken {
        return PreviewData::Error(format!(
            "Broken symlink -> {}",
            entry
                .symlink_target
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "?".to_string())
        ));
    }

    if entry.is_dir {
        return match read_directory_with_cache(&entry.path, cache) {
            Ok(mut children) => {
                if !show_hidden {
                    children.retain(|c| !c.is_hidden);
                }
                sort_entries(&mut children, SortField::Name, false, true);
                let total_count = children.len();
                children.truncate(PREVIEW_DIR_LIMIT);
                PreviewData::Directory {
                    entries: children,
                    total_count,
                }
            }
            Err(err) => PreviewData::Error(err.to_string()),
        };
    }

    let mut file = match File::open(&entry.path) {
        Ok(f) => f,
        Err(err) => return PreviewData::Error(format!("Cannot read file: {err}")),
    };

    let mut buf = vec![0_u8; PREVIEW_MAX_BYTES];
    let n = match file.read(&mut buf) {
        Ok(n) => n,
        Err(err) => return PreviewData::Error(format!("Read error: {err}")),
    };
    buf.truncate(n);

    let probe_len = buf.len().min(1024);
    if buf[..probe_len].contains(&0) {
        return PreviewData::Binary { size: entry.size };
    }

    let text = String::from_utf8_lossy(&buf);
    let mut lines = Vec::new();
    let mut truncated = entry.size as usize > n;
    for line in text.lines() {
        if lines.len() >= PREVIEW_MAX_LINES {
            truncated = true;
            break;
        }
        lines.push(line.replace('\t', "    "));
    }

    PreviewData::Text { lines, truncated }
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

fn sort_entries(entries: &mut [FileEntry], field: SortField, reverse: bool, dirs_first: bool) {
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
        use crate::config::ConfigFile;
        use ratatui::style::Color;

        let root = make_test_tree();
        let custom = Theme::from_toml_str("accent = \"#123456\"").unwrap();
        let loaded = LoadedConfig {
            config: ConfigFile {
                show_borders: Some(false),
                show_footer: Some(false),
                show_hidden: Some(true),
                show_permissions: Some(true),
                show_size: Some(false),
                max_rows: Some(15),
                ..ConfigFile::default()
            },
            custom_theme: Some(custom),
            warning: None,
        };

        // Without CLI overrides: config wins and custom theme becomes default
        let args = Args::try_parse_from(["lsi", root.to_str().unwrap()]).unwrap();
        let mut app = App::new_with_config(args, loaded.clone()).unwrap();
        assert!(!app.show_borders);
        assert!(!app.show_footer);
        assert!(app.show_hidden);
        assert!(app.show_permissions);
        assert!(!app.show_size);
        assert_eq!(app.max_rows, 15);
        assert_eq!(app.theme_name, ThemeName::Custom);
        assert_eq!(app.theme().accent, Color::Rgb(0x12, 0x34, 0x56));

        // Cycling moves through built-in themes and back to Custom
        app.handle_key(KeyEvent::from(KeyCode::Char('T')));
        assert_eq!(app.theme_name, ThemeName::EzaDark);
        for _ in 0..ThemeName::BUILTIN.len() {
            app.handle_key(KeyEvent::from(KeyCode::Char('T')));
        }
        assert_eq!(app.theme_name, ThemeName::Custom);

        // Explicit CLI flags override config.toml and theme.toml default
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
        let app_cli = App::new_with_config(cli_override, loaded).unwrap();
        assert!(app_cli.show_borders);
        assert!(app_cli.show_footer);
        assert!(app_cli.show_size);
        assert_eq!(app_cli.theme_name, ThemeName::Nord);
        assert_eq!(app_cli.max_rows, 30);

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
}
