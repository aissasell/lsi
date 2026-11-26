use std::fs::File;
use std::io::{self, Read, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
#[cfg(feature = "syntax-highlight")]
use std::sync::OnceLock;
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

#[cfg(feature = "image-preview")]
use image::imageops::FilterType;
#[cfg(feature = "image-preview")]
use image::ImageReader;
use lru::LruCache;
#[cfg(feature = "image-preview")]
use ratatui::layout::Size;
use ratatui::style::{Color, Modifier};
#[cfg(feature = "image-preview")]
use ratatui_image::{
    picker::{Picker, ProtocolType},
    protocol::Protocol,
    Resize,
};
use serde::Deserialize;
#[cfg(feature = "syntax-highlight")]
use syntect::{
    easy::HighlightLines,
    highlighting::{FontStyle, Theme as SyntectTheme, ThemeSet},
    parsing::{SyntaxReference, SyntaxSet},
};
#[cfg(feature = "syntax-highlight")]
use two_face::theme::{EmbeddedLazyThemeSet, EmbeddedThemeName};

use crate::cli::SortField;
use crate::fs_entry::{read_directory_with_cache, FileEntry, IdentityCache};
use crate::theme::ThemeName;
use crate::video_preview::{
    build_video_preview, is_ambiguous_video_extension, is_video_extension, looks_like_mpeg_ts_video,
};
#[cfg(feature = "image-preview")]
use crate::video_preview::{has_cached_thumbnail, VIDEO_DEBOUNCE_MS};

pub const PREVIEW_MAX_LINES: usize = 200;
pub const PREVIEW_MAX_BYTES: usize = 64 * 1024;
pub const PREVIEW_DIR_LIMIT: usize = 100;
#[cfg(feature = "syntax-highlight")]
const HIGHLIGHT_MAX_LINE_LEN: usize = 1000;
#[cfg(feature = "image-preview")]
const IMAGE_MAX_PIXELS: u64 = 100_000_000;
#[cfg(feature = "image-preview")]
const IMAGE_DEBOUNCE_MS: u64 = 35;
const PREVIEW_CACHE_CAP: usize = 24;
const DEFAULT_MAX_FILE_BYTES: u64 = 20 * 1024 * 1024;

/// Terminal graphics protocol preference for image previews.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImageProtocol {
    #[default]
    Auto,
    Kitty,
    Sixel,
    Iterm2,
    Halfblocks,
    #[serde(alias = "none")]
    Off,
}

/// Configured syntax highlighting colorscheme for text previews.
#[derive(Clone, Default)]
pub enum SyntaxThemeSelection {
    #[default]
    Auto,
    #[cfg(feature = "syntax-highlight")]
    Embedded(EmbeddedThemeName),
    #[cfg(feature = "syntax-highlight")]
    Custom {
        name: String,
        theme: Arc<SyntectTheme>,
    },
}

impl std::fmt::Debug for SyntaxThemeSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto => f.write_str("Auto"),
            #[cfg(feature = "syntax-highlight")]
            Self::Embedded(name) => f.debug_tuple("Embedded").field(name).finish(),
            #[cfg(feature = "syntax-highlight")]
            Self::Custom { name, .. } => f.debug_struct("Custom").field("name", name).finish(),
        }
    }
}

impl PartialEq for SyntaxThemeSelection {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Auto, Self::Auto) => true,
            #[cfg(feature = "syntax-highlight")]
            (Self::Embedded(a), Self::Embedded(b)) => a == b,
            #[cfg(feature = "syntax-highlight")]
            (
                Self::Custom {
                    name: na,
                    theme: ta,
                },
                Self::Custom {
                    name: nb,
                    theme: tb,
                },
            ) => na == nb && Arc::ptr_eq(ta, tb),
            #[cfg(feature = "syntax-highlight")]
            _ => false,
        }
    }
}

impl Eq for SyntaxThemeSelection {}

impl std::hash::Hash for SyntaxThemeSelection {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::Auto => {}
            #[cfg(feature = "syntax-highlight")]
            Self::Embedded(name) => name.hash(state),
            #[cfg(feature = "syntax-highlight")]
            Self::Custom { name, theme } => {
                name.hash(state);
                Arc::as_ptr(theme).hash(state);
            }
        }
    }
}

/// Runtime preview behavior configured via `config.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PreviewConfig {
    pub syntax_highlight: bool,
    pub syntax_theme: SyntaxThemeSelection,
    pub image_preview: bool,
    pub video_preview: bool,
    pub image_protocol: ImageProtocol,
    pub max_file_bytes: u64,
}

impl Default for PreviewConfig {
    fn default() -> Self {
        Self {
            syntax_highlight: true,
            syntax_theme: SyntaxThemeSelection::Auto,
            image_preview: true,
            video_preview: true,
            image_protocol: ImageProtocol::Auto,
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
        }
    }
}

/// Owned styled text fragment for a previewed line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledSegment {
    pub text: String,
    pub fg: Option<Color>,
    pub modifiers: Modifier,
}

/// A single rendered line of styled text segments.
pub type PreviewLine = Vec<StyledSegment>;

/// Cached or in-flight content for the side-by-side preview pane.
#[derive(Clone)]
pub enum PreviewData {
    Empty,
    Directory {
        entries: Vec<FileEntry>,
        total_count: usize,
    },
    Text {
        lines: Vec<PreviewLine>,
        truncated: bool,
        syntax_name: Option<String>,
    },
    #[cfg(feature = "image-preview")]
    Image {
        protocol: Protocol,
        width: u32,
        height: u32,
        format_label: String,
    },
    Metadata {
        title: String,
        rows: Vec<(String, String)>,
        note: Option<String>,
    },
    Binary {
        size: u64,
        detail: Option<String>,
    },
    Error(String),
}

impl std::fmt::Debug for PreviewData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => f.write_str("Empty"),
            Self::Directory { total_count, .. } => f
                .debug_struct("Directory")
                .field("total_count", total_count)
                .finish(),
            Self::Text {
                lines,
                truncated,
                syntax_name,
            } => f
                .debug_struct("Text")
                .field("lines", &lines.len())
                .field("truncated", truncated)
                .field("syntax_name", syntax_name)
                .finish(),
            #[cfg(feature = "image-preview")]
            Self::Image {
                width,
                height,
                format_label,
                ..
            } => f
                .debug_struct("Image")
                .field("width", width)
                .field("height", height)
                .field("format_label", format_label)
                .finish(),
            Self::Metadata { title, rows, note } => f
                .debug_struct("Metadata")
                .field("title", title)
                .field("rows", &rows.len())
                .field("note", note)
                .finish(),
            Self::Binary { size, detail } => f
                .debug_struct("Binary")
                .field("size", size)
                .field("detail", detail)
                .finish(),
            Self::Error(msg) => f.debug_tuple("Error").field(msg).finish(),
        }
    }
}

/// Preview build request dispatched to the background worker.
#[derive(Debug, Clone)]
pub struct PreviewRequest {
    pub req_id: u64,
    pub entry: FileEntry,
    pub pane_size: (u16, u16),
    pub show_hidden: bool,
    pub ui_theme_name: ThemeName,
    pub is_light_theme: bool,
    pub config: PreviewConfig,
}

/// Result produced asynchronously by the preview worker thread.
pub struct PreviewResponse {
    pub req_id: u64,
    pub path: PathBuf,
    pub data: PreviewData,
}

#[cfg(feature = "image-preview")]
pub type ImagePickerHandle = Picker;

#[cfg(not(feature = "image-preview"))]
pub type ImagePickerHandle = ();

#[cfg(feature = "image-preview")]
pub enum WorkerCommand {
    Request(Box<PreviewRequest>),
    UpdatePicker(Option<Arc<ImagePickerHandle>>),
    ClearCache,
    Shutdown,
}

#[cfg(not(feature = "image-preview"))]
pub enum WorkerCommand {
    Request(Box<PreviewRequest>),
    ClearCache,
    Shutdown,
}

/// Manages the dedicated background `std::thread` for non-blocking file & image previews.
pub struct PreviewWorker {
    tx: Sender<WorkerCommand>,
    rx: Receiver<PreviewResponse>,
    handle: Option<JoinHandle<()>>,
}

impl PreviewWorker {
    /// Spawns the background preview worker thread using an optional initial image picker.
    pub fn spawn(initial_picker: Option<Arc<ImagePickerHandle>>) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<WorkerCommand>();
        let (resp_tx, resp_rx) = mpsc::channel::<PreviewResponse>();
        let handle = thread::Builder::new()
            .name("lsi-preview".to_string())
            .spawn(move || worker_loop(cmd_rx, resp_tx, initial_picker))
            .expect("failed to spawn preview worker thread");

        Self {
            tx: cmd_tx,
            rx: resp_rx,
            handle: Some(handle),
        }
    }

    pub fn send_request(&self, req: PreviewRequest) {
        let _ = self.tx.send(WorkerCommand::Request(Box::new(req)));
    }

    #[cfg(feature = "image-preview")]
    pub fn update_picker(&self, picker: Option<Arc<ImagePickerHandle>>) {
        let _ = self.tx.send(WorkerCommand::UpdatePicker(picker));
    }

    #[cfg(not(feature = "image-preview"))]
    pub fn update_picker(&self, _picker: Option<Arc<ImagePickerHandle>>) {}

    pub fn clear_cache(&self) {
        let _ = self.tx.send(WorkerCommand::ClearCache);
    }

    pub fn try_recv(&self) -> Result<PreviewResponse, mpsc::TryRecvError> {
        self.rx.try_recv()
    }
}

impl Drop for PreviewWorker {
    fn drop(&mut self) {
        let _ = self.tx.send(WorkerCommand::Shutdown);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SyntaxThemeCacheKey {
    Auto { ui_theme: ThemeName, is_light: bool },
    #[cfg(feature = "syntax-highlight")]
    Embedded(EmbeddedThemeName),
    #[cfg(feature = "syntax-highlight")]
    Custom { name: String, ptr: usize },
}

impl SyntaxThemeCacheKey {
    fn from_request(req: &PreviewRequest) -> Self {
        match &req.config.syntax_theme {
            SyntaxThemeSelection::Auto => Self::Auto {
                ui_theme: req.ui_theme_name,
                is_light: req.is_light_theme,
            },
            #[cfg(feature = "syntax-highlight")]
            SyntaxThemeSelection::Embedded(name) => Self::Embedded(*name),
            #[cfg(feature = "syntax-highlight")]
            SyntaxThemeSelection::Custom { name, theme } => Self::Custom {
                name: name.clone(),
                ptr: Arc::as_ptr(theme) as usize,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PreviewCacheKey {
    path: PathBuf,
    modified: Option<SystemTime>,
    size: u64,
    pane_size: (u16, u16),
    show_hidden: bool,
    syntax_theme: SyntaxThemeCacheKey,
    syntax_highlight: bool,
    image_preview: bool,
    video_preview: bool,
    image_protocol: ImageProtocol,
    max_file_bytes: u64,
}

impl PreviewCacheKey {
    fn from_request(req: &PreviewRequest) -> Self {
        let pane_size = if is_image_request(req) || is_video_thumbnail_request(req) {
            req.pane_size
        } else {
            (0, 0)
        };
        Self {
            path: req.entry.path.clone(),
            modified: req.entry.modified,
            size: req.entry.size,
            pane_size,
            show_hidden: req.show_hidden,
            syntax_theme: SyntaxThemeCacheKey::from_request(req),
            syntax_highlight: req.config.syntax_highlight,
            image_preview: req.config.image_preview,
            video_preview: req.config.video_preview,
            image_protocol: req.config.image_protocol,
            max_file_bytes: req.config.max_file_bytes,
        }
    }
}

fn is_image_request(req: &PreviewRequest) -> bool {
    #[cfg(feature = "image-preview")]
    {
        !req.entry.is_dir
            && !req.entry.symlink_broken
            && req.config.image_preview
            && req.config.image_protocol != ImageProtocol::Off
            && is_image_extension(&req.entry.extension)
    }
    #[cfg(not(feature = "image-preview"))]
    {
        let _ = req;
        false
    }
}

fn is_video_entry(entry: &FileEntry) -> bool {
    if !is_video_extension(&entry.extension) {
        return false;
    }
    if is_ambiguous_video_extension(&entry.extension) {
        return looks_like_mpeg_ts_video(&entry.path);
    }
    true
}

fn is_video_thumbnail_request(req: &PreviewRequest) -> bool {
    #[cfg(feature = "image-preview")]
    {
        !req.entry.is_dir
            && !req.entry.symlink_broken
            && req.config.video_preview
            && req.config.image_preview
            && req.config.image_protocol != ImageProtocol::Off
            && is_video_entry(&req.entry)
    }
    #[cfg(not(feature = "image-preview"))]
    {
        let _ = req;
        false
    }
}

fn apply_control_cmd(
    cmd: WorkerCommand,
    cache: &mut LruCache<PreviewCacheKey, PreviewData>,
    picker: &mut Option<Arc<ImagePickerHandle>>,
) -> bool {
    let _ = picker;
    match cmd {
        #[cfg(feature = "image-preview")]
        WorkerCommand::UpdatePicker(next) => {
            *picker = next;
            cache.clear();
            true
        }
        WorkerCommand::ClearCache => {
            cache.clear();
            true
        }
        WorkerCommand::Shutdown => false,
        WorkerCommand::Request(_) => unreachable!(),
    }
}

enum DrainedCommand {
    Request(Option<Box<PreviewRequest>>),
    Shutdown,
}

fn drain_to_latest(
    cmd_rx: &Receiver<WorkerCommand>,
    mut latest: Option<Box<PreviewRequest>>,
    cache: &mut LruCache<PreviewCacheKey, PreviewData>,
    picker: &mut Option<Arc<ImagePickerHandle>>,
) -> DrainedCommand {
    while let Ok(cmd) = cmd_rx.try_recv() {
        match cmd {
            WorkerCommand::Request(req) => latest = Some(req),
            other => {
                if !apply_control_cmd(other, cache, picker) {
                    return DrainedCommand::Shutdown;
                }
            }
        }
    }
    DrainedCommand::Request(latest)
}

fn worker_loop(
    cmd_rx: Receiver<WorkerCommand>,
    resp_tx: Sender<PreviewResponse>,
    mut picker: Option<Arc<ImagePickerHandle>>,
) {
    let cap = NonZeroUsize::new(PREVIEW_CACHE_CAP).expect("PREVIEW_CACHE_CAP > 0");
    let mut cache: LruCache<PreviewCacheKey, PreviewData> = LruCache::new(cap);
    let mut identity_cache = IdentityCache::new();
    let mut pending_cmd: Option<WorkerCommand> = None;

    loop {
        let first_cmd = match pending_cmd.take() {
            Some(cmd) => Ok(cmd),
            None => cmd_rx.recv(),
        };
        let next_req = match first_cmd {
            Ok(WorkerCommand::Request(req)) => Some(req),
            Ok(WorkerCommand::Shutdown) | Err(_) => return,
            Ok(other) => {
                if !apply_control_cmd(other, &mut cache, &mut picker) {
                    return;
                }
                match drain_to_latest(&cmd_rx, None, &mut cache, &mut picker) {
                    DrainedCommand::Request(req) => req,
                    DrainedCommand::Shutdown => return,
                }
            }
        };

        #[cfg_attr(not(feature = "image-preview"), allow(unused_mut))]
        let mut req = match drain_to_latest(&cmd_rx, next_req, &mut cache, &mut picker) {
            DrainedCommand::Request(Some(req)) => req,
            DrainedCommand::Request(None) => continue,
            DrainedCommand::Shutdown => return,
        };

        #[cfg_attr(not(feature = "image-preview"), allow(unused_mut))]
        let mut cache_key = PreviewCacheKey::from_request(&req);
        if let Some(cached) = cache.get(&cache_key).cloned() {
            if resp_tx
                .send(PreviewResponse {
                    req_id: req.req_id,
                    path: req.entry.path,
                    data: cached,
                })
                .is_err()
            {
                return;
            }
            continue;
        }

        // Briefly debounce raster image and uncached video decoding during fast cursor scrolling.
        #[cfg(feature = "image-preview")]
        if is_image_request(&req) || is_video_thumbnail_request(&req) {
            loop {
                let delay_ms = if is_video_thumbnail_request(&req) {
                    if has_cached_thumbnail(&req.entry) {
                        IMAGE_DEBOUNCE_MS
                    } else {
                        VIDEO_DEBOUNCE_MS
                    }
                } else {
                    IMAGE_DEBOUNCE_MS
                };
                match cmd_rx.recv_timeout(Duration::from_millis(delay_ms)) {
                    Ok(WorkerCommand::Request(newer)) => {
                        req = newer;
                        match drain_to_latest(&cmd_rx, None, &mut cache, &mut picker) {
                            DrainedCommand::Request(Some(newer2)) => req = newer2,
                            DrainedCommand::Request(None) => {}
                            DrainedCommand::Shutdown => return,
                        }
                        if !is_image_request(&req) && !is_video_thumbnail_request(&req) {
                            break;
                        }
                    }
                    Ok(WorkerCommand::Shutdown) | Err(RecvTimeoutError::Disconnected) => {
                        return;
                    }
                    Ok(other) => {
                        if !apply_control_cmd(other, &mut cache, &mut picker) {
                            return;
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => break,
                }
            }
            cache_key = PreviewCacheKey::from_request(&req);
            if let Some(cached) = cache.get(&cache_key).cloned() {
                if resp_tx
                    .send(PreviewResponse {
                        req_id: req.req_id,
                        path: req.entry.path,
                        data: cached,
                    })
                    .is_err()
                {
                    return;
                }
                continue;
            }
        }

        #[cfg(not(feature = "image-preview"))]
        {
            let _ = RecvTimeoutError::Timeout;
            let _ = Duration::from_millis(0);
        }

        let opts = PreviewBuildOpts {
            show_hidden: req.show_hidden,
            ui_theme_name: req.ui_theme_name,
            is_light_theme: req.is_light_theme,
            pane_size: req.pane_size,
            config: &req.config,
            picker: picker.as_deref(),
        };
        let mut is_cancelled = || -> bool {
            if pending_cmd.is_some() {
                return true;
            }
            match cmd_rx.try_recv() {
                Ok(cmd) => {
                    pending_cmd = Some(cmd);
                    true
                }
                Err(mpsc::TryRecvError::Disconnected) => true,
                Err(mpsc::TryRecvError::Empty) => false,
            }
        };
        let Some(data) =
            build_preview_with_cancel(&req.entry, &mut identity_cache, &opts, &mut is_cancelled)
        else {
            continue;
        };
        if !matches!(data, PreviewData::Error(_)) {
            cache.put(cache_key, data.clone());
        }

        if resp_tx
            .send(PreviewResponse {
                req_id: req.req_id,
                path: req.entry.path,
                data,
            })
            .is_err()
        {
            return;
        }
    }
}

/// Options used when synchronously constructing a preview.
pub struct PreviewBuildOpts<'a> {
    pub show_hidden: bool,
    pub ui_theme_name: ThemeName,
    pub is_light_theme: bool,
    pub pane_size: (u16, u16),
    pub config: &'a PreviewConfig,
    #[cfg_attr(not(feature = "image-preview"), allow(dead_code))]
    pub picker: Option<&'a ImagePickerHandle>,
}

/// Returns `true` for raster image file extensions handled by `image`.
#[cfg_attr(not(feature = "image-preview"), allow(dead_code))]
pub fn is_image_extension(ext: &str) -> bool {
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico"
    )
}

/// Creates a terminal image picker using the requested protocol (optionally querying stdio once).
#[cfg(feature = "image-preview")]
pub fn create_image_picker(
    protocol: ImageProtocol,
    allow_stdio_query: bool,
) -> Option<ImagePickerHandle> {
    if protocol == ImageProtocol::Off {
        return None;
    }
    if protocol == ImageProtocol::Halfblocks {
        return Some(Picker::halfblocks());
    }

    let mut picker = if allow_stdio_query {
        Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks())
    } else {
        Picker::halfblocks()
    };

    match protocol {
        ImageProtocol::Auto | ImageProtocol::Halfblocks | ImageProtocol::Off => {}
        ImageProtocol::Kitty => picker.set_protocol_type(ProtocolType::Kitty),
        ImageProtocol::Sixel => picker.set_protocol_type(ProtocolType::Sixel),
        ImageProtocol::Iterm2 => picker.set_protocol_type(ProtocolType::Iterm2),
    }

    Some(picker)
}

#[cfg(not(feature = "image-preview"))]
pub fn create_image_picker(
    _protocol: ImageProtocol,
    _allow_stdio_query: bool,
) -> Option<ImagePickerHandle> {
    None
}

/// Reports whether the given picker emits Kitty graphics protocol sequences.
#[cfg(feature = "image-preview")]
pub fn picker_uses_kitty(picker: &ImagePickerHandle) -> bool {
    picker.protocol_type() == ProtocolType::Kitty
}

#[cfg(not(feature = "image-preview"))]
pub fn picker_uses_kitty(_picker: &ImagePickerHandle) -> bool {
    false
}

/// Clears any Kitty graphics overlays emitted inside `Viewport::Inline` so no artifacts
/// remain over scrollback or the shell prompt when resizing, hiding preview, or exiting.
pub fn clear_terminal_graphics(out: &mut impl Write, uses_kitty: bool) -> io::Result<()> {
    #[cfg(feature = "image-preview")]
    {
        if uses_kitty {
            // APC Kitty Graphics command: a=d (delete), d=A (all visible placements & images), q=2 (quiet).
            out.write_all(b"\x1b_Ga=d,d=A,q=2\x1b\\")?;
            out.flush()?;
        }
    }
    #[cfg(not(feature = "image-preview"))]
    {
        let _ = (out, uses_kitty);
    }
    Ok(())
}

/// Synchronously builds preview data for `entry`.
#[cfg(all(test, any(feature = "syntax-highlight", feature = "image-preview")))]
pub fn build_preview_sync(
    entry: &FileEntry,
    cache: &mut IdentityCache,
    opts: &PreviewBuildOpts<'_>,
) -> PreviewData {
    build_preview_with_cancel(entry, cache, opts, &mut || false).unwrap_or(PreviewData::Empty)
}

fn build_preview_with_cancel(
    entry: &FileEntry,
    cache: &mut IdentityCache,
    opts: &PreviewBuildOpts<'_>,
    is_cancelled: &mut dyn FnMut() -> bool,
) -> Option<PreviewData> {
    if entry.symlink_broken {
        return Some(PreviewData::Error(format!(
            "Broken symlink -> {}",
            entry
                .symlink_target
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "?".to_string())
        )));
    }

    if entry.is_dir {
        return Some(match read_directory_with_cache(&entry.path, cache) {
            Ok(mut children) => {
                if !opts.show_hidden {
                    children.retain(|c| !c.is_hidden);
                }
                crate::app::sort_entries(&mut children, SortField::Name, false, true);
                let total_count = children.len();
                children.truncate(PREVIEW_DIR_LIMIT);
                PreviewData::Directory {
                    entries: children,
                    total_count,
                }
            }
            Err(err) => PreviewData::Error(err.to_string()),
        });
    }

    #[cfg(feature = "image-preview")]
    if opts.config.image_preview
        && opts.config.image_protocol != ImageProtocol::Off
        && is_image_extension(&entry.extension)
    {
        if let Some(picker) = opts.picker {
            return Some(build_image_preview(
                entry,
                picker,
                opts.pane_size,
                opts.config.max_file_bytes,
            ));
        }
    }

    if is_video_entry(entry) {
        return build_video_preview(entry, opts, is_cancelled);
    }

    let mut file = match File::open(&entry.path) {
        Ok(f) => f,
        Err(err) => return Some(PreviewData::Error(format!("Cannot read file: {err}"))),
    };

    let mut buf = vec![0_u8; PREVIEW_MAX_BYTES];
    let n = match file.read(&mut buf) {
        Ok(n) => n,
        Err(err) => return Some(PreviewData::Error(format!("Read error: {err}"))),
    };
    buf.truncate(n);

    let probe_len = buf.len().min(1024);
    if buf[..probe_len].contains(&0) {
        return Some(PreviewData::Binary {
            size: entry.size,
            detail: None,
        });
    }

    let text = String::from_utf8_lossy(&buf);
    let mut raw_lines = Vec::new();
    let mut truncated = entry.size as usize > n;
    for line in text.lines() {
        if raw_lines.len() >= PREVIEW_MAX_LINES {
            truncated = true;
            break;
        }
        raw_lines.push(line.replace('\t', "    "));
    }

    let (lines, syntax_name) = if opts.config.syntax_highlight {
        highlight_or_plain(
            &entry.path,
            &entry.extension,
            &raw_lines,
            opts.ui_theme_name,
            opts.is_light_theme,
            &opts.config.syntax_theme,
        )
    } else {
        let _ = (
            opts.ui_theme_name,
            opts.is_light_theme,
            &opts.config.syntax_theme,
            opts.pane_size,
        );
        (plain_lines(&raw_lines), None)
    };

    Some(PreviewData::Text {
        lines,
        truncated,
        syntax_name,
    })
}

/// Resizes and encodes an already-decoded image into the terminal protocol for `pane_size`.
#[cfg(feature = "image-preview")]
pub(crate) fn encode_image_for_pane(
    mut dyn_img: image::DynamicImage,
    width: u32,
    height: u32,
    format_label: String,
    picker: &ImagePickerHandle,
    pane_size: (u16, u16),
) -> PreviewData {
    let cols = pane_size.0.max(2);
    let rows = pane_size.1.max(2);
    let font = picker.font_size();
    let target_px_w = u32::from(cols).saturating_mul(u32::from(font.width.max(1)));
    let target_px_h = u32::from(rows).saturating_mul(u32::from(font.height.max(1)));
    if dyn_img.width() > target_px_w || dyn_img.height() > target_px_h {
        dyn_img = dyn_img.resize(target_px_w, target_px_h, FilterType::Triangle);
    }

    match picker.new_protocol(
        dyn_img,
        Size::new(cols, rows),
        Resize::Fit(Some(FilterType::Triangle)),
    ) {
        Ok(protocol) => PreviewData::Image {
            protocol,
            width,
            height,
            format_label,
        },
        Err(err) => PreviewData::Error(format!("Image encode error: {err}")),
    }
}

#[cfg(feature = "image-preview")]
fn build_image_preview(
    entry: &FileEntry,
    picker: &ImagePickerHandle,
    pane_size: (u16, u16),
    max_file_bytes: u64,
) -> PreviewData {
    if entry.size > max_file_bytes {
        let max_mb = (max_file_bytes / (1024 * 1024)).max(1);
        return PreviewData::Binary {
            size: entry.size,
            detail: Some(format!("Image skipped (exceeds {max_mb} MiB limit)")),
        };
    }

    let probe_reader = match ImageReader::open(&entry.path) {
        Ok(r) => match r.with_guessed_format() {
            Ok(r) => r,
            Err(err) => return PreviewData::Error(format!("Image read error: {err}")),
        },
        Err(err) => return PreviewData::Error(format!("Cannot read image: {err}")),
    };

    let format_label = probe_reader
        .format()
        .map(|fmt| format!("{fmt:?}").to_ascii_uppercase())
        .unwrap_or_else(|| "IMAGE".to_string());

    let (width, height) = match probe_reader.into_dimensions() {
        Ok(dims) => dims,
        Err(err) => {
            return PreviewData::Binary {
                size: entry.size,
                detail: Some(format!("{format_label} header unreadable: {err}")),
            };
        }
    };

    let total_pixels = u64::from(width).saturating_mul(u64::from(height));
    if width == 0 || height == 0 || total_pixels > IMAGE_MAX_PIXELS {
        return PreviewData::Binary {
            size: entry.size,
            detail: Some(format!(
                "{format_label} {width}x{height} (too large to decode)"
            )),
        };
    }

    let decode_reader = match ImageReader::open(&entry.path) {
        Ok(r) => match r.with_guessed_format() {
            Ok(r) => r,
            Err(err) => return PreviewData::Error(format!("Image read error: {err}")),
        },
        Err(err) => return PreviewData::Error(format!("Cannot read image: {err}")),
    };

    let dyn_img = match decode_reader.decode() {
        Ok(img) => img,
        Err(err) => {
            return PreviewData::Binary {
                size: entry.size,
                detail: Some(format!("{format_label} {width}x{height} ({err})")),
            };
        }
    };

    encode_image_for_pane(dyn_img, width, height, format_label, picker, pane_size)
}

fn plain_lines(raw_lines: &[String]) -> Vec<PreviewLine> {
    raw_lines
        .iter()
        .map(|line| {
            vec![StyledSegment {
                text: line.clone(),
                fg: None,
                modifiers: Modifier::empty(),
            }]
        })
        .collect()
}

#[cfg(feature = "syntax-highlight")]
fn non_plain(syntax: Option<&SyntaxReference>) -> Option<&SyntaxReference> {
    syntax.filter(|s| s.name != "Plain Text")
}

#[cfg(feature = "syntax-highlight")]
fn find_first_syntax<'a>(
    ss: &'a SyntaxSet,
    candidates: &[&str],
) -> Option<&'a SyntaxReference> {
    candidates.iter().find_map(|token| {
        non_plain(ss.find_syntax_by_token(token))
            .or_else(|| non_plain(ss.find_syntax_by_extension(token)))
    })
}

#[cfg(feature = "syntax-highlight")]
fn fallback_candidates_for_filename(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "cargo.lock" | "uv.lock" | "poetry.lock" | "pdm.lock" | "pipfile" => {
            Some(&["toml", "ini"])
        }
        "flake.lock" | ".prettierrc" | ".eslintrc" | ".babelrc" | ".swcrc" => Some(&["json"]),
        ".gitignore" | ".dockerignore" | ".npmignore" | ".prettierignore" | ".eslintignore"
        | ".hgignore" => Some(&["gitignore", "sh"]),
        ".editorconfig" | ".gitconfig" | ".gitmodules" | ".npmrc" | ".yarnrc" => Some(&["ini"]),
        ".env" => Some(&["env", "dotenv", "sh"]),
        _ if name.starts_with(".env.") => Some(&["env", "dotenv", "sh"]),
        "dockerfile" | "containerfile" => Some(&["Dockerfile", "sh"]),
        _ if name.starts_with("dockerfile.") || name.starts_with("containerfile.") => {
            Some(&["Dockerfile", "sh"])
        }
        "justfile" | ".justfile" => Some(&["just", "Makefile"]),
        "cmakelists.txt" => Some(&["cmake", "CMake"]),
        "gemfile" | "rakefile" | "podfile" | "vagrantfile" | "brewfile" => Some(&["rb"]),
        ".bashrc" | ".bash_profile" | ".bash_logout" | ".profile" | ".zshrc" | ".zshenv"
        | ".zprofile" | ".zlogin" | ".zlogout" | ".kshrc" => Some(&["sh"]),
        _ => None,
    }
}

#[cfg(feature = "syntax-highlight")]
fn fallback_candidates_for_extension(ext: &str) -> Option<&'static [&'static str]> {
    match ext {
        "mdx" => Some(&["md"]),
        "mjs" | "cjs" => Some(&["js"]),
        "jsx" => Some(&["tsx", "js"]),
        "mts" | "cts" => Some(&["ts", "js"]),
        "astro" => Some(&["html"]),
        "jsonc" | "json5" | "webmanifest" => Some(&["json", "js"]),
        "env" => Some(&["env", "dotenv", "sh"]),
        "conf" | "cfg" | "properties" | "editorconfig" => Some(&["ini"]),
        "svg" | "plist" | "xhtml" | "csproj" | "fsproj" | "vbproj" | "props" | "targets" => {
            Some(&["xml"])
        }
        "gradle" => Some(&["groovy"]),
        "zsh" => Some(&["sh"]),
        "dockerfile" | "containerfile" => Some(&["Dockerfile", "sh"]),
        "just" => Some(&["just", "Makefile"]),
        _ => None,
    }
}

#[cfg(feature = "syntax-highlight")]
fn find_fallback_syntax<'a>(
    ss: &'a SyntaxSet,
    path: &Path,
    ext: &str,
) -> Option<&'a SyntaxReference> {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        let lower_name = name.to_ascii_lowercase();
        if let Some(candidates) = fallback_candidates_for_filename(&lower_name) {
            if let Some(syntax) = find_first_syntax(ss, candidates) {
                return Some(syntax);
            }
        }
    }

    if !ext.is_empty() {
        let lower_ext = ext.to_ascii_lowercase();
        if let Some(candidates) = fallback_candidates_for_extension(&lower_ext) {
            if let Some(syntax) = find_first_syntax(ss, candidates) {
                return Some(syntax);
            }
        }
    }

    None
}

#[cfg(feature = "syntax-highlight")]
fn resolve_syntax<'a>(
    ss: &'a SyntaxSet,
    path: &Path,
    ext: &str,
    first_line: Option<&str>,
) -> Option<&'a SyntaxReference> {
    let path_str = path.to_string_lossy();
    non_plain(ss.find_syntax_by_path(&path_str))
        .or_else(|| {
            if ext.is_empty() {
                None
            } else {
                non_plain(ss.find_syntax_by_extension(ext))
            }
        })
        .or_else(|| {
            path.file_name()
                .and_then(|n| n.to_str())
                .and_then(|name| non_plain(ss.find_syntax_by_extension(name)))
        })
        .or_else(|| first_line.and_then(|first| non_plain(ss.find_syntax_by_first_line(first))))
        .or_else(|| find_fallback_syntax(ss, path, ext))
}

#[cfg(feature = "syntax-highlight")]
fn normalize_theme_key(raw: &str) -> String {
    raw.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Parses an embedded `two-face` syntax theme name (case- and punctuation-insensitive).
#[cfg(feature = "syntax-highlight")]
pub fn parse_embedded_syntax_theme(raw: &str) -> Option<EmbeddedThemeName> {
    let key = normalize_theme_key(raw);
    if key.is_empty() {
        return None;
    }
    if key == "leet" {
        return Some(EmbeddedThemeName::Leet);
    }
    EmbeddedLazyThemeSet::theme_names()
        .iter()
        .copied()
        .find(|name| normalize_theme_key(name.as_name()) == key)
}

/// Returns the default `two-face` syntax highlighting theme paired with an `lsi` UI theme.
#[cfg(feature = "syntax-highlight")]
pub const fn auto_embedded_theme(ui_theme: ThemeName, is_light_theme: bool) -> EmbeddedThemeName {
    match ui_theme {
        ThemeName::EzaDark => EmbeddedThemeName::Base16OceanDark,
        ThemeName::Catppuccin => EmbeddedThemeName::CatppuccinMocha,
        ThemeName::Dracula => EmbeddedThemeName::Dracula,
        ThemeName::Nord => EmbeddedThemeName::Nord,
        ThemeName::Gruvbox => EmbeddedThemeName::GruvboxDark,
        ThemeName::TokyoNight => EmbeddedThemeName::TwoDark,
        ThemeName::SolarizedLight => EmbeddedThemeName::SolarizedLight,
        ThemeName::Custom => {
            if is_light_theme {
                EmbeddedThemeName::InspiredGithub
            } else {
                EmbeddedThemeName::Base16OceanDark
            }
        }
    }
}

/// Loads a `.tmTheme` file from disk using `syntect`'s pure-Rust plist parser.
#[cfg(feature = "syntax-highlight")]
pub fn load_custom_syntax_theme(path: &Path) -> Result<Arc<SyntectTheme>, String> {
    ThemeSet::get_theme(path)
        .map(Arc::new)
        .map_err(|err| err.to_string())
}

#[cfg(feature = "syntax-highlight")]
fn highlight_or_plain(
    path: &Path,
    ext: &str,
    raw_lines: &[String],
    ui_theme: ThemeName,
    is_light_theme: bool,
    syntax_theme: &SyntaxThemeSelection,
) -> (Vec<PreviewLine>, Option<String>) {
    static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
    static THEME_SET: OnceLock<EmbeddedLazyThemeSet> = OnceLock::new();

    if raw_lines.is_empty()
        || raw_lines
            .first()
            .is_some_and(|line| line.len() > HIGHLIGHT_MAX_LINE_LEN)
    {
        return (plain_lines(raw_lines), None);
    }

    let ss = SYNTAX_SET.get_or_init(two_face::syntax::extra_newlines);
    let ts = THEME_SET.get_or_init(two_face::theme::extra);

    let Some(syntax) = resolve_syntax(ss, path, ext, raw_lines.first().map(String::as_str))
    else {
        return (plain_lines(raw_lines), None);
    };

    let syn_theme = match syntax_theme {
        SyntaxThemeSelection::Auto => ts.get(auto_embedded_theme(ui_theme, is_light_theme)),
        SyntaxThemeSelection::Embedded(name) => ts.get(*name),
        SyntaxThemeSelection::Custom { theme, .. } => theme.as_ref(),
    };

    let mut highlighter = HighlightLines::new(syntax, syn_theme);
    let mut out = Vec::with_capacity(raw_lines.len());
    for line in raw_lines {
        if line.len() > HIGHLIGHT_MAX_LINE_LEN {
            out.push(vec![StyledSegment {
                text: line.clone(),
                fg: None,
                modifiers: Modifier::empty(),
            }]);
            continue;
        }

        let mut with_nl = String::with_capacity(line.len() + 1);
        with_nl.push_str(line);
        with_nl.push('\n');

        match highlighter.highlight_line(&with_nl, ss) {
            Ok(ranges) => {
                let mut segments = Vec::with_capacity(ranges.len());
                for (style, slice) in ranges {
                    let clean = slice.trim_end_matches(['\r', '\n']);
                    if clean.is_empty() {
                        continue;
                    }
                    let fg = (style.foreground.a > 0).then_some(Color::Rgb(
                        style.foreground.r,
                        style.foreground.g,
                        style.foreground.b,
                    ));
                    segments.push(StyledSegment {
                        text: clean.to_string(),
                        fg,
                        modifiers: font_style_to_modifier(style.font_style),
                    });
                }
                out.push(segments);
            }
            Err(_) => {
                out.push(vec![StyledSegment {
                    text: line.clone(),
                    fg: None,
                    modifiers: Modifier::empty(),
                }]);
            }
        }
    }

    (out, Some(syntax.name.clone()))
}

#[cfg(not(feature = "syntax-highlight"))]
fn highlight_or_plain(
    _path: &Path,
    _ext: &str,
    raw_lines: &[String],
    _ui_theme: ThemeName,
    _is_light_theme: bool,
    _syntax_theme: &SyntaxThemeSelection,
) -> (Vec<PreviewLine>, Option<String>) {
    (plain_lines(raw_lines), None)
}

#[cfg(feature = "syntax-highlight")]
const fn font_style_to_modifier(style: FontStyle) -> Modifier {
    let mut mods = Modifier::empty();
    if style.contains(FontStyle::BOLD) {
        mods = mods.union(Modifier::BOLD);
    }
    if style.contains(FontStyle::ITALIC) {
        mods = mods.union(Modifier::ITALIC);
    }
    if style.contains(FontStyle::UNDERLINE) {
        mods = mods.union(Modifier::UNDERLINED);
    }
    mods
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs_entry::read_directory_with_cache;
    use std::fs;
    use std::time::UNIX_EPOCH;

    fn make_temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lsi_preview_test_{tag}_{}_{}",
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

    #[cfg(feature = "syntax-highlight")]
    #[test]
    fn highlights_rust_and_bails_out_on_minified_long_line() {
        let dir = make_temp_dir("syntax");
        fs::write(dir.join("lib.rs"), "pub fn answer() -> u32 {\n    42\n}\n").unwrap();
        let minified = format!("const DATA = \"{}\";\n", "x".repeat(1200));
        fs::write(dir.join("bundle.min.js"), minified).unwrap();

        let mut cache = IdentityCache::new();
        let entries = read_directory_with_cache(&dir, &mut cache).unwrap();
        let rs_entry = entries.iter().find(|e| e.name == "lib.rs").unwrap();
        let min_entry = entries.iter().find(|e| e.name == "bundle.min.js").unwrap();

        let cfg = PreviewConfig::default();
        let opts = PreviewBuildOpts {
            show_hidden: false,
            ui_theme_name: ThemeName::EzaDark,
            is_light_theme: false,
            pane_size: (40, 20),
            config: &cfg,
            picker: None,
        };

        match build_preview_sync(rs_entry, &mut cache, &opts) {
            PreviewData::Text {
                lines,
                truncated,
                syntax_name,
            } => {
                assert!(!truncated);
                assert_eq!(syntax_name.as_deref(), Some("Rust"));
                assert_eq!(lines.len(), 3);
                assert!(lines[0].iter().any(|seg| seg.fg.is_some()));
            }
            other => panic!("expected highlighted Text preview, got {other:?}"),
        }

        match build_preview_sync(min_entry, &mut cache, &opts) {
            PreviewData::Text {
                lines, syntax_name, ..
            } => {
                assert!(syntax_name.is_none());
                assert_eq!(lines.len(), 1);
                assert!(lines[0].iter().all(|seg| seg.fg.is_none()));
            }
            other => panic!("expected plain fallback for minified js, got {other:?}"),
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(feature = "syntax-highlight")]
    #[test]
    fn highlights_extended_formats_and_common_aliases() {
        let dir = make_temp_dir("extended_syntax");
        let highlighted_cases = [
            (
                "config.toml",
                "[package]\nname = \"lsi\"\nversion = \"1.3.0\"\n",
                Some("toml"),
            ),
            (
                "app.ts",
                "export const answer: number = 42;\n",
                Some("typescript"),
            ),
            (
                "mod.mts",
                "export type Id = string;\n",
                Some("typescript"),
            ),
            (
                "Dockerfile",
                "FROM rust:1.88\nRUN cargo build --release\n",
                None,
            ),
            ("config.fish", "set -gx EDITOR nvim\n", None),
            ("flake.nix", "{ description = \"lsi\"; }\n", None),
            (
                "component.jsx",
                "export const App = () => <div className=\"ok\">hi</div>;\n",
                None,
            ),
            ("main.zig", "pub fn main() void {}\n", None),
            ("page.mdx", "# Heading\n\nSome **bold** text.\n", None),
            (
                "astro.config.mjs",
                "export default { site: \"https://example.com\" };\n",
                None,
            ),
            (
                "Cargo.lock",
                "[[package]]\nname = \"lsi\"\nversion = \"1.3.0\"\n",
                Some("toml"),
            ),
            (".env.local", "RUST_LOG=debug\n", None),
            ("script", "#!/usr/bin/env python3\nprint(\"hi\")\n", None),
        ];

        for (name, content, _) in &highlighted_cases {
            fs::write(dir.join(name), content).unwrap();
        }
        fs::write(dir.join("notes.txt"), "just plain notes\n").unwrap();

        let mut mpeg_ts = vec![0_u8; 564];
        mpeg_ts[0] = 0x47;
        mpeg_ts[188] = 0x47;
        mpeg_ts[376] = 0x47;
        fs::write(dir.join("stream.ts"), &mpeg_ts).unwrap();

        let mut cache = IdentityCache::new();
        let entries = read_directory_with_cache(&dir, &mut cache).unwrap();
        let cfg = PreviewConfig::default();
        let opts = PreviewBuildOpts {
            show_hidden: true,
            ui_theme_name: ThemeName::EzaDark,
            is_light_theme: false,
            pane_size: (40, 20),
            config: &cfg,
            picker: None,
        };

        for (name, _, expected_substr) in &highlighted_cases {
            let entry = entries
                .iter()
                .find(|e| e.name == *name)
                .unwrap_or_else(|| panic!("missing test entry {name}"));
            match build_preview_sync(entry, &mut cache, &opts) {
                PreviewData::Text {
                    lines, syntax_name, ..
                } => {
                    let syntax_label = syntax_name
                        .as_deref()
                        .unwrap_or_else(|| panic!("expected syntax_name for {name}"));
                    if let Some(substr) = expected_substr {
                        assert!(
                            syntax_label.to_ascii_lowercase().contains(substr),
                            "expected syntax '{syntax_label}' for {name} to contain '{substr}'"
                        );
                    }
                    assert!(
                        lines.iter().flatten().any(|seg| seg.fg.is_some()),
                        "expected colored highlight segments for {name} (syntax: {syntax_label})"
                    );
                }
                other => panic!("expected highlighted Text preview for {name}, got {other:?}"),
            }
        }

        let notes_entry = entries.iter().find(|e| e.name == "notes.txt").unwrap();
        match build_preview_sync(notes_entry, &mut cache, &opts) {
            PreviewData::Text {
                lines, syntax_name, ..
            } => {
                assert!(syntax_name.is_none());
                assert_eq!(lines.len(), 1);
                assert!(lines[0].iter().all(|seg| seg.fg.is_none()));
            }
            other => panic!("expected plain Text preview for notes.txt, got {other:?}"),
        }

        let stream_entry = entries.iter().find(|e| e.name == "stream.ts").unwrap();
        match build_preview_sync(stream_entry, &mut cache, &opts) {
            PreviewData::Metadata { title, .. } => {
                assert_eq!(title, "Video file");
            }
            other => panic!("expected Video metadata preview for binary stream.ts, got {other:?}"),
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(feature = "syntax-highlight")]
    #[test]
    fn parses_embedded_syntax_theme_names_and_aliases() {
        assert_eq!(
            parse_embedded_syntax_theme("catppuccin-mocha"),
            Some(EmbeddedThemeName::CatppuccinMocha)
        );
        assert_eq!(
            parse_embedded_syntax_theme("Catppuccin Mocha"),
            Some(EmbeddedThemeName::CatppuccinMocha)
        );
        assert_eq!(
            parse_embedded_syntax_theme("Solarized (light)"),
            Some(EmbeddedThemeName::SolarizedLight)
        );
        assert_eq!(
            parse_embedded_syntax_theme("solarized-light"),
            Some(EmbeddedThemeName::SolarizedLight)
        );
        assert_eq!(
            parse_embedded_syntax_theme("gruvbox-dark"),
            Some(EmbeddedThemeName::GruvboxDark)
        );
        assert_eq!(
            parse_embedded_syntax_theme("base16-ocean.dark"),
            Some(EmbeddedThemeName::Base16OceanDark)
        );
        assert_eq!(
            parse_embedded_syntax_theme("base16-ocean-dark"),
            Some(EmbeddedThemeName::Base16OceanDark)
        );
        assert_eq!(
            parse_embedded_syntax_theme("1337"),
            Some(EmbeddedThemeName::Leet)
        );
        assert_eq!(
            parse_embedded_syntax_theme("leet"),
            Some(EmbeddedThemeName::Leet)
        );
        assert_eq!(parse_embedded_syntax_theme("does-not-exist"), None);
        assert_eq!(parse_embedded_syntax_theme("   "), None);
    }

    #[cfg(feature = "syntax-highlight")]
    #[test]
    fn applies_auto_embedded_and_custom_tmtheme_colorschemes() {
        assert_eq!(
            auto_embedded_theme(ThemeName::EzaDark, false),
            EmbeddedThemeName::Base16OceanDark
        );

        let dir = make_temp_dir("syntax_themes");
        fs::write(
            dir.join("test.toml"),
            "[package]\nname = \"lsi\"\nversion = \"1.3.0\"\n",
        )
        .unwrap();
        let tmtheme_path = dir.join("magenta.tmTheme");
        fs::write(
            &tmtheme_path,
            r##"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>name</key>
  <string>MagentaTest</string>
  <key>settings</key>
  <array>
    <dict>
      <key>settings</key>
      <dict>
        <key>background</key>
        <string>#111111</string>
        <key>foreground</key>
        <string>#ff00aa</string>
      </dict>
    </dict>
  </array>
</dict>
</plist>
"##,
        )
        .unwrap();

        let mut cache = IdentityCache::new();
        let entries = read_directory_with_cache(&dir, &mut cache).unwrap();
        let toml_entry = entries.iter().find(|e| e.name == "test.toml").unwrap();

        let mut render_with = |ui_theme_name: ThemeName,
                           is_light_theme: bool,
                           syntax_theme: SyntaxThemeSelection| {
            let cfg = PreviewConfig {
                syntax_theme,
                ..PreviewConfig::default()
            };
            let opts = PreviewBuildOpts {
                show_hidden: false,
                ui_theme_name,
                is_light_theme,
                pane_size: (40, 20),
                config: &cfg,
                picker: None,
            };
            match build_preview_sync(toml_entry, &mut cache, &opts) {
                PreviewData::Text { lines, .. } => lines,
                other => panic!("expected Text preview, got {other:?}"),
            }
        };

        let catppuccin_lines = render_with(ThemeName::Catppuccin, false, SyntaxThemeSelection::Auto);
        let gruvbox_lines = render_with(ThemeName::Gruvbox, false, SyntaxThemeSelection::Auto);
        let solarized_light_lines =
            render_with(ThemeName::SolarizedLight, true, SyntaxThemeSelection::Auto);
        assert_ne!(catppuccin_lines, gruvbox_lines);
        assert_ne!(catppuccin_lines, solarized_light_lines);

        let dracula_auto = render_with(ThemeName::Dracula, false, SyntaxThemeSelection::Auto);
        let dracula_pinned = render_with(
            ThemeName::SolarizedLight,
            true,
            SyntaxThemeSelection::Embedded(EmbeddedThemeName::Dracula),
        );
        assert_eq!(dracula_auto, dracula_pinned);

        let custom_theme = load_custom_syntax_theme(&tmtheme_path).expect("valid .tmTheme");
        let custom_lines = render_with(
            ThemeName::EzaDark,
            false,
            SyntaxThemeSelection::Custom {
                name: "magenta".to_string(),
                theme: custom_theme,
            },
        );
        assert!(custom_lines
            .iter()
            .flatten()
            .any(|seg| seg.fg == Some(Color::Rgb(0xff, 0x00, 0xaa))));

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(feature = "image-preview")]
    #[test]
    fn decodes_png_images_and_respects_size_and_protocol_limits() {
        use image::{ImageBuffer, Rgba};

        let dir = make_temp_dir("image");
        let img_path = dir.join("sample.png");
        let buf: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(32, 24, Rgba([60, 140, 220, 255]));
        buf.save(&img_path).unwrap();

        let mut cache = IdentityCache::new();
        let entries = read_directory_with_cache(&dir, &mut cache).unwrap();
        let png_entry = entries.iter().find(|e| e.name == "sample.png").unwrap();

        let picker = create_image_picker(ImageProtocol::Halfblocks, false).unwrap();
        let cfg = PreviewConfig::default();
        let opts = PreviewBuildOpts {
            show_hidden: false,
            ui_theme_name: ThemeName::EzaDark,
            is_light_theme: false,
            pane_size: (30, 14),
            config: &cfg,
            picker: Some(&picker),
        };

        match build_preview_sync(png_entry, &mut cache, &opts) {
            PreviewData::Image {
                width,
                height,
                format_label,
                ..
            } => {
                assert_eq!((width, height), (32, 24));
                assert_eq!(format_label, "PNG");
            }
            other => panic!("expected Image preview, got {other:?}"),
        }

        // Tiny max_file_bytes limit skips decode cleanly
        let small_cfg = PreviewConfig {
            max_file_bytes: 10,
            ..PreviewConfig::default()
        };
        let small_opts = PreviewBuildOpts {
            show_hidden: false,
            ui_theme_name: ThemeName::EzaDark,
            is_light_theme: false,
            pane_size: (30, 14),
            config: &small_cfg,
            picker: Some(&picker),
        };
        match build_preview_sync(png_entry, &mut cache, &small_opts) {
            PreviewData::Binary { detail, .. } => {
                assert!(detail.unwrap_or_default().contains("Image skipped"));
            }
            other => panic!("expected Binary skipped notice, got {other:?}"),
        }

        // ImageProtocol::Off disables picker creation
        assert!(create_image_picker(ImageProtocol::Off, false).is_none());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn worker_coalesces_requests_and_caches_results() {
        let dir = make_temp_dir("worker");
        fs::write(dir.join("first.txt"), "hello 1\n").unwrap();
        fs::write(dir.join("second.txt"), "hello 2\n").unwrap();

        let mut cache = IdentityCache::new();
        let entries = read_directory_with_cache(&dir, &mut cache).unwrap();
        let first = entries
            .iter()
            .find(|e| e.name == "first.txt")
            .unwrap()
            .clone();
        let second = entries
            .iter()
            .find(|e| e.name == "second.txt")
            .unwrap()
            .clone();

        let worker = PreviewWorker::spawn(None);
        worker.send_request(PreviewRequest {
            req_id: 1,
            entry: first,
            pane_size: (40, 16),
            show_hidden: false,
            ui_theme_name: ThemeName::EzaDark,
            is_light_theme: false,
            config: PreviewConfig::default(),
        });
        worker.send_request(PreviewRequest {
            req_id: 2,
            entry: second.clone(),
            pane_size: (40, 16),
            show_hidden: false,
            ui_theme_name: ThemeName::EzaDark,
            is_light_theme: false,
            config: PreviewConfig::default(),
        });

        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        let mut received = Vec::new();
        while std::time::Instant::now() < deadline {
            if let Ok(resp) = worker.try_recv() {
                let is_latest = resp.req_id == 2;
                received.push(resp);
                if is_latest {
                    break;
                }
            }
            thread::sleep(Duration::from_millis(5));
        }

        let last = received.last().expect("worker returned response");
        assert_eq!(last.req_id, 2);
        assert_eq!(last.path, second.path);

        // Sending the same entry with req_id 3 hits cache rapidly
        worker.send_request(PreviewRequest {
            req_id: 3,
            entry: second,
            pane_size: (40, 16),
            show_hidden: false,
            ui_theme_name: ThemeName::EzaDark,
            is_light_theme: false,
            config: PreviewConfig::default(),
        });
        let cached_deadline = std::time::Instant::now() + Duration::from_secs(1);
        let mut cached_resp = None;
        while std::time::Instant::now() < cached_deadline {
            if let Ok(resp) = worker.try_recv() {
                if resp.req_id == 3 {
                    cached_resp = Some(resp);
                    break;
                }
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(cached_resp.expect("cached response").req_id, 3);

        let _ = fs::remove_dir_all(&dir);
    }
}
