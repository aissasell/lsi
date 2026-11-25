#[cfg(any(feature = "image-preview", test))]
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io;
#[cfg(feature = "image-preview")]
use std::io::Read;
#[cfg(all(unix, any(feature = "image-preview", test)))]
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};
#[cfg(feature = "image-preview")]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "image-preview")]
use image::ImageReader;

use crate::fs_entry::{format_size, format_time, FileEntry};
#[cfg(any(feature = "image-preview", test))]
use crate::preview::ImageProtocol;
#[cfg(feature = "image-preview")]
use crate::preview::{encode_image_for_pane, ImagePickerHandle};
use crate::preview::{PreviewBuildOpts, PreviewData};

/// Debounce delay in milliseconds for uncached video thumbnail extraction.
#[cfg(feature = "image-preview")]
pub const VIDEO_DEBOUNCE_MS: u64 = 100;
#[cfg(feature = "image-preview")]
const VIDEO_THUMB_TIMEOUT: Duration = Duration::from_secs(3);
const VIDEO_PROBE_TIMEOUT: Duration = Duration::from_millis(1500);
#[cfg(feature = "image-preview")]
const DISK_CACHE_MAX_FILES: usize = 200;
const CHILD_POLL_INTERVAL: Duration = Duration::from_millis(10);

static TEMP_SEQ: AtomicU64 = AtomicU64::new(1);

/// Returns `true` for common container extensions handled by external video thumbnailers.
pub fn is_video_extension(ext: &str) -> bool {
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "mp4"
            | "mkv"
            | "webm"
            | "mov"
            | "avi"
            | "m4v"
            | "flv"
            | "wmv"
            | "mpg"
            | "mpeg"
            | "ogv"
            | "3gp"
            | "ts"
            | "mts"
            | "m2ts"
    )
}

/// CLI executable names or paths used for video thumbnail extraction and metadata probing.
#[derive(Debug, Clone, Copy)]
pub struct VideoToolPaths<'a> {
    #[cfg_attr(not(feature = "image-preview"), allow(dead_code))]
    pub ffmpegthumbnailer: &'a str,
    #[cfg_attr(not(feature = "image-preview"), allow(dead_code))]
    pub ffmpeg: &'a str,
    pub ffprobe: &'a str,
}

impl Default for VideoToolPaths<'static> {
    fn default() -> Self {
        Self {
            ffmpegthumbnailer: "ffmpegthumbnailer",
            ffmpeg: "ffmpeg",
            ffprobe: "ffprobe",
        }
    }
}

enum SubprocessOutcome {
    Exited(ExitStatus),
    NotFound,
    TimedOut,
    Cancelled,
    SpawnError(#[cfg_attr(not(feature = "image-preview"), allow(dead_code))] io::Error),
}

fn run_command_cancellable(
    cmd: &mut Command,
    timeout: Duration,
    is_cancelled: &mut dyn FnMut() -> bool,
) -> SubprocessOutcome {
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return SubprocessOutcome::NotFound,
        Err(err) => return SubprocessOutcome::SpawnError(err),
    };

    let started = Instant::now();
    loop {
        if is_cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            return SubprocessOutcome::Cancelled;
        }
        match child.try_wait() {
            Ok(Some(status)) => return SubprocessOutcome::Exited(status),
            Ok(None) => {}
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                return SubprocessOutcome::SpawnError(err);
            }
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return SubprocessOutcome::TimedOut;
        }
        thread::sleep(CHILD_POLL_INTERVAL);
    }
}

struct TempFileGuard(Option<PathBuf>);

impl TempFileGuard {
    fn new(path: PathBuf) -> Self {
        Self(Some(path))
    }

    #[cfg(feature = "image-preview")]
    fn defuse(&mut self) {
        self.0 = None;
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if let Some(path) = self.0.take() {
            let _ = fs::remove_file(path);
        }
    }
}

fn unique_temp_path(dir: &Path, prefix: &str, ext: &str) -> PathBuf {
    let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    dir.join(format!(".tmp_{prefix}_{}_{seq}.{ext}", std::process::id()))
}

/// Ensures relative filenames starting with `-` cannot be interpreted as CLI options.
fn safe_cli_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        Path::new(".").join(path)
    }
}

#[cfg(feature = "image-preview")]
fn is_non_empty_file(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|meta| meta.is_file() && meta.len() > 0)
}

#[cfg(any(feature = "image-preview", test))]
const MD5_SHIFTS: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

#[cfg(any(feature = "image-preview", test))]
const MD5_K: [u32; 64] = [
    0xd76a_a478,
    0xe8c7_b756,
    0x2420_70db,
    0xc1bd_ceee,
    0xf57c_0faf,
    0x4787_c62a,
    0xa830_4613,
    0xfd46_9501,
    0x6980_98d8,
    0x8b44_f7af,
    0xffff_5bb1,
    0x895c_d7be,
    0x6b90_1122,
    0xfd98_7193,
    0xa679_438e,
    0x49b4_0821,
    0xf61e_2562,
    0xc040_b340,
    0x265e_5a51,
    0xe9b6_c7aa,
    0xd62f_105d,
    0x0244_1453,
    0xd8a1_e681,
    0xe7d3_fbc8,
    0x21e1_cde6,
    0xc337_07d6,
    0xf4d5_0d87,
    0x455a_14ed,
    0xa9e3_e905,
    0xfcef_a3f8,
    0x676f_02d9,
    0x8d2a_4c8a,
    0xfffa_3942,
    0x8771_f681,
    0x6d9d_6122,
    0xfde5_380c,
    0xa4be_ea44,
    0x4bde_cfa9,
    0xf6bb_4b60,
    0xbebf_bc70,
    0x289b_7ec6,
    0xeaa1_27fa,
    0xd4ef_3085,
    0x0488_1d05,
    0xd9d4_d039,
    0xe6db_99e5,
    0x1fa2_7cf8,
    0xc4ac_5665,
    0xf429_2244,
    0x432a_ff97,
    0xab94_23a7,
    0xfc93_a039,
    0x655b_59c3,
    0x8f0c_cc92,
    0xffef_f47d,
    0x8584_5dd1,
    0x6fa8_7e4f,
    0xfe2c_e6e0,
    0xa301_4314,
    0x4e08_11a1,
    0xf753_7e82,
    0xbd3a_f235,
    0x2ad7_d2bb,
    0xeb86_d391,
];

/// Computes lowercase RFC 1321 MD5 digest hex for FreeDesktop thumbnail URI keys.
#[cfg(any(feature = "image-preview", test))]
pub(crate) fn md5_hex(data: &[u8]) -> String {
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_le_bytes());

    let mut a0: u32 = 0x6745_2301;
    let mut b0: u32 = 0xefcd_ab89;
    let mut c0: u32 = 0x98ba_dcfe;
    let mut d0: u32 = 0x1032_5476;

    for chunk in msg.chunks_exact(64) {
        let mut m = [0_u32; 16];
        for (idx, word) in m.iter_mut().enumerate() {
            let off = idx * 4;
            *word =
                u32::from_le_bytes([chunk[off], chunk[off + 1], chunk[off + 2], chunk[off + 3]]);
        }

        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i {
                0..=15 => ((b & c) | ((!b) & d), i),
                16..=31 => ((d & b) | ((!d) & c), (5 * i + 1) % 16),
                32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | (!d)), (7 * i) % 16),
            };
            let temp = d;
            d = c;
            c = b;
            let sum = a.wrapping_add(f).wrapping_add(MD5_K[i]).wrapping_add(m[g]);
            b = b.wrapping_add(sum.rotate_left(MD5_SHIFTS[i]));
            a = temp;
        }

        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }

    let mut out = String::with_capacity(32);
    for word in [a0, b0, c0, d0] {
        for byte in word.to_le_bytes() {
            use std::fmt::Write;
            let _ = write!(out, "{byte:02x}");
        }
    }
    out
}

/// Converts a local filesystem path into a percent-encoded `file://` URI.
#[cfg(any(feature = "image-preview", test))]
pub(crate) fn file_uri_for_path(path: &Path) -> String {
    let raw_bytes: std::borrow::Cow<'_, [u8]> = {
        #[cfg(unix)]
        {
            std::borrow::Cow::Borrowed(path.as_os_str().as_bytes())
        }
        #[cfg(not(unix))]
        {
            std::borrow::Cow::Owned(path.to_string_lossy().into_owned().into_bytes())
        }
    };

    let mut out = String::from("file://");
    for &byte in raw_bytes.as_ref() {
        let keep = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/');
        if keep {
            out.push(byte as char);
        } else {
            use std::fmt::Write;
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// Resolves `$XDG_CACHE_HOME` (requiring an absolute non-empty path per the XDG spec),
/// defaulting to `$HOME/.cache` when `$XDG_CACHE_HOME` is unset, empty, or relative.
#[cfg(any(feature = "image-preview", test))]
fn resolve_cache_base(xdg_cache_home: Option<&OsStr>, home: Option<&OsStr>) -> Option<PathBuf> {
    if let Some(xdg) = xdg_cache_home.filter(|v| !v.is_empty()) {
        let path = PathBuf::from(xdg);
        if path.is_absolute() {
            return Some(path);
        }
    }
    home.filter(|v| !v.is_empty())
        .map(|h| PathBuf::from(h).join(".cache"))
}

#[cfg(feature = "image-preview")]
fn default_cache_base() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .or_else(|| std::env::var_os("USERPROFILE").filter(|v| !v.is_empty()));
    resolve_cache_base(std::env::var_os("XDG_CACHE_HOME").as_deref(), home.as_deref())
}

#[cfg(any(feature = "image-preview", test))]
fn lsi_thumbnail_cache_dir(cache_base: Option<&Path>) -> Option<PathBuf> {
    cache_base.map(|base| base.join("lsi").join("thumbnails"))
}

#[cfg(any(feature = "image-preview", test))]
fn freedesktop_thumbnail_dir(cache_base: Option<&Path>) -> Option<PathBuf> {
    cache_base.map(|base| base.join("thumbnails"))
}

#[cfg(feature = "image-preview")]
fn entry_mtime_secs(entry: &FileEntry) -> u64 {
    entry
        .modified
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(feature = "image-preview")]
fn canonical_uri_md5(path: &Path) -> String {
    let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    md5_hex(file_uri_for_path(&canonical).as_bytes())
}

#[cfg(feature = "image-preview")]
fn lsi_cache_stem(md5: &str, entry: &FileEntry) -> String {
    format!("{md5}_{}_{}", entry_mtime_secs(entry), entry.size)
}

#[cfg(feature = "image-preview")]
fn find_cached_thumbnail_in(
    entry: &FileEntry,
    lsi_cache_dir: Option<&Path>,
    freedesktop_dir: Option<&Path>,
) -> Option<PathBuf> {
    let md5 = canonical_uri_md5(&entry.path);

    if let Some(cache_dir) = lsi_cache_dir {
        let stem = lsi_cache_stem(&md5, entry);
        for ext in ["jpg", "png"] {
            let candidate = cache_dir.join(format!("{stem}.{ext}"));
            if is_non_empty_file(&candidate) {
                return Some(candidate);
            }
        }
    }

    if let Some(fd_dir) = freedesktop_dir {
        let filename = format!("{md5}.png");
        for bucket in ["x-large", "large", "xx-large", "normal"] {
            let candidate = fd_dir.join(bucket).join(&filename);
            if is_non_empty_file(&candidate) && is_freedesktop_thumb_fresh(&candidate, entry) {
                return Some(candidate);
            }
        }
    }

    None
}

/// Fast synchronous check used by `PreviewWorker` to pick short debounce when a thumbnail is cached.
#[cfg(feature = "image-preview")]
pub fn has_cached_thumbnail(entry: &FileEntry) -> bool {
    let base = default_cache_base();
    let lsi_dir = lsi_thumbnail_cache_dir(base.as_deref());
    let fd_dir = freedesktop_thumbnail_dir(base.as_deref());
    find_cached_thumbnail_in(entry, lsi_dir.as_deref(), fd_dir.as_deref()).is_some()
}

#[cfg(feature = "image-preview")]
fn parse_png_thumb_metadata(path: &Path) -> Option<(Option<u64>, Option<u64>)> {
    let mut file = File::open(path).ok()?;
    let mut buf = vec![0_u8; 64 * 1024];
    let n = file.read(&mut buf).ok()?;
    buf.truncate(n);

    const PNG_SIG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if buf.len() < 8 || &buf[..8] != PNG_SIG {
        return None;
    }

    let mut thumb_mtime = None;
    let mut thumb_size = None;
    let mut pos = 8_usize;

    while pos + 8 <= buf.len() {
        let len = u32::from_be_bytes([buf[pos], buf[pos + 1], buf[pos + 2], buf[pos + 3]]) as usize;
        let chunk_type = &buf[pos + 4..pos + 8];
        if chunk_type == b"IDAT" || chunk_type == b"IEND" {
            break;
        }
        let data_start = pos + 8;
        let Some(data_end) = data_start.checked_add(len) else {
            break;
        };
        if data_end > buf.len() {
            break;
        }
        if chunk_type == b"tEXt" {
            let payload = &buf[data_start..data_end];
            if let Some(sep) = payload.iter().position(|&b| b == 0) {
                let key = &payload[..sep];
                let val_bytes = &payload[sep + 1..];
                if let Ok(val_str) = std::str::from_utf8(val_bytes) {
                    if let Ok(parsed) = val_str.trim().parse::<u64>() {
                        if key == b"Thumb::MTime" {
                            thumb_mtime = Some(parsed);
                        } else if key == b"Thumb::Size" {
                            thumb_size = Some(parsed);
                        }
                    }
                }
            }
        }
        let Some(next_pos) = data_end.checked_add(4) else {
            break;
        };
        pos = next_pos;
    }

    Some((thumb_mtime, thumb_size))
}

#[cfg(feature = "image-preview")]
fn is_freedesktop_thumb_fresh(thumb_path: &Path, entry: &FileEntry) -> bool {
    let Some((thumb_mtime, thumb_size)) = parse_png_thumb_metadata(thumb_path) else {
        return false;
    };

    if let Some(recorded_mtime) = thumb_mtime {
        if entry.modified.is_some() && recorded_mtime != entry_mtime_secs(entry) {
            return false;
        }
        if let Some(recorded_size) = thumb_size {
            if recorded_size != entry.size {
                return false;
            }
        }
        return true;
    }

    match (
        fs::metadata(thumb_path).and_then(|m| m.modified()),
        entry.modified,
    ) {
        (Ok(thumb_fs_mtime), Some(file_mtime)) => thumb_fs_mtime >= file_mtime,
        _ => true,
    }
}

#[cfg(feature = "image-preview")]
enum GenerateOutcome {
    Cancelled,
    ToolsMissing,
    Failed(String),
}

#[cfg(feature = "image-preview")]
fn prune_disk_cache(dir: &Path, max_files: usize) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut cached: Vec<(PathBuf, SystemTime)> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let path = entry.path();
        let is_thumb_ext = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("jpg") || e.eq_ignore_ascii_case("png"));
        if !is_thumb_ext {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            if meta.is_file() {
                cached.push((path, meta.modified().unwrap_or(UNIX_EPOCH)));
            }
        }
    }

    if cached.len() <= max_files {
        return;
    }
    let keep_target = (max_files * 4) / 5;
    cached.sort_by_key(|(_, mtime)| *mtime);
    for (path, _) in cached.iter().take(cached.len().saturating_sub(keep_target)) {
        let _ = fs::remove_file(path);
    }
}

#[cfg(feature = "image-preview")]
fn decode_image_file_for_pane(
    img_path: &Path,
    picker: &ImagePickerHandle,
    pane_size: (u16, u16),
) -> Result<PreviewData, String> {
    let reader = ImageReader::open(img_path)
        .map_err(|err| format!("Cannot read thumbnail: {err}"))?
        .with_guessed_format()
        .map_err(|err| format!("Thumbnail read error: {err}"))?;

    let format_label = reader
        .format()
        .map(|fmt| format!("{fmt:?}").to_ascii_uppercase())
        .unwrap_or_else(|| "VIDEO".to_string());

    let dyn_img = reader
        .decode()
        .map_err(|err| format!("Thumbnail decode error: {err}"))?;
    let (width, height) = (dyn_img.width(), dyn_img.height());
    if width == 0 || height == 0 {
        return Err("Empty thumbnail image".to_string());
    }

    Ok(encode_image_for_pane(
        dyn_img,
        width,
        height,
        format_label,
        picker,
        pane_size,
    ))
}

#[cfg(feature = "image-preview")]
fn generate_and_load_via_cli(
    entry: &FileEntry,
    lsi_cache_dir: Option<&Path>,
    picker: &ImagePickerHandle,
    pane_size: (u16, u16),
    tools: &VideoToolPaths<'_>,
    is_cancelled: &mut dyn FnMut() -> bool,
) -> Result<PreviewData, GenerateOutcome> {
    let cache_dir_ready = lsi_cache_dir.and_then(|dir| fs::create_dir_all(dir).ok().map(|()| dir));
    let work_dir = cache_dir_ready.map_or_else(std::env::temp_dir, Path::to_path_buf);
    let tmp_path = unique_temp_path(&work_dir, "vthumb", "jpg");
    let mut tmp_guard = TempFileGuard::new(tmp_path.clone());
    let input = safe_cli_path(&entry.path);

    let mut saw_tool = false;
    let mut last_reason: Option<String> = None;

    // 1. Try ffmpegthumbnailer first (fast purpose-built CLI thumbnailer).
    let mut thumb_cmd = Command::new(tools.ffmpegthumbnailer);
    thumb_cmd
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .arg("-i")
        .arg(&input)
        .arg("-o")
        .arg(&tmp_path)
        .args(["-s", "512", "-t", "10%", "-c", "jpeg", "-q", "8"]);

    match run_command_cancellable(&mut thumb_cmd, VIDEO_THUMB_TIMEOUT, is_cancelled) {
        SubprocessOutcome::Cancelled => return Err(GenerateOutcome::Cancelled),
        SubprocessOutcome::NotFound | SubprocessOutcome::SpawnError(_) => {}
        SubprocessOutcome::TimedOut => {
            saw_tool = true;
            last_reason = Some("ffmpegthumbnailer timed out".to_string());
            let _ = fs::remove_file(&tmp_path);
        }
        SubprocessOutcome::Exited(status) => {
            saw_tool = true;
            if status.success() && is_non_empty_file(&tmp_path) {
                return finalize_generated_thumb(
                    entry,
                    &tmp_path,
                    &mut tmp_guard,
                    cache_dir_ready,
                    picker,
                    pane_size,
                );
            }
            last_reason = Some("ffmpegthumbnailer failed".to_string());
            let _ = fs::remove_file(&tmp_path);
        }
    }

    if is_cancelled() {
        return Err(GenerateOutcome::Cancelled);
    }

    // 2. Fallback to ffmpeg with fast keyframe input seek at 5s, retrying at 0s for short clips.
    for seek_ts in ["00:00:05", "00:00:00"] {
        if is_cancelled() {
            return Err(GenerateOutcome::Cancelled);
        }

        let mut ff_cmd = Command::new(tools.ffmpeg);
        ff_cmd
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .args([
                "-nostdin",
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-protocol_whitelist",
                "file,pipe",
                "-threads",
                "2",
                "-ss",
                seek_ts,
                "-i",
            ])
            .arg(&input)
            .args([
                "-an",
                "-sn",
                "-dn",
                "-frames:v",
                "1",
                "-vf",
                "scale=min(512\\,iw):min(512\\,ih):force_original_aspect_ratio=decrease:flags=fast_bilinear",
                "-strict",
                "unofficial",
                "-q:v",
                "4",
            ])
            .arg(&tmp_path);

        match run_command_cancellable(&mut ff_cmd, VIDEO_THUMB_TIMEOUT, is_cancelled) {
            SubprocessOutcome::Cancelled => return Err(GenerateOutcome::Cancelled),
            SubprocessOutcome::NotFound => {
                if !saw_tool {
                    return Err(GenerateOutcome::ToolsMissing);
                }
                return Err(GenerateOutcome::Failed(
                    last_reason.unwrap_or_else(|| "thumbnail tool failed".to_string()),
                ));
            }
            SubprocessOutcome::TimedOut => {
                return Err(GenerateOutcome::Failed("ffmpeg timed out".to_string()));
            }
            SubprocessOutcome::SpawnError(err) => {
                last_reason = Some(format!("ffmpeg spawn error: {err}"));
                break;
            }
            SubprocessOutcome::Exited(status) => {
                saw_tool = true;
                if status.success() && is_non_empty_file(&tmp_path) {
                    return finalize_generated_thumb(
                        entry,
                        &tmp_path,
                        &mut tmp_guard,
                        cache_dir_ready,
                        picker,
                        pane_size,
                    );
                }
                let _ = fs::remove_file(&tmp_path);
                last_reason = Some("ffmpeg could not extract frame".to_string());
            }
        }
    }

    if !saw_tool {
        Err(GenerateOutcome::ToolsMissing)
    } else {
        Err(GenerateOutcome::Failed(last_reason.unwrap_or_else(|| {
            "thumbnail generation failed".to_string()
        })))
    }
}

#[cfg(feature = "image-preview")]
fn finalize_generated_thumb(
    entry: &FileEntry,
    tmp_path: &Path,
    tmp_guard: &mut TempFileGuard,
    cache_dir: Option<&Path>,
    picker: &ImagePickerHandle,
    pane_size: (u16, u16),
) -> Result<PreviewData, GenerateOutcome> {
    let mut final_path = tmp_path.to_path_buf();
    if let Some(dir) = cache_dir {
        let md5 = canonical_uri_md5(&entry.path);
        let cache_target = dir.join(format!("{}.jpg", lsi_cache_stem(&md5, entry)));
        if fs::rename(tmp_path, &cache_target).is_ok() {
            tmp_guard.defuse();
            prune_disk_cache(dir, DISK_CACHE_MAX_FILES);
            final_path = cache_target;
        }
    }

    match decode_image_file_for_pane(&final_path, picker, pane_size) {
        Ok(PreviewData::Image {
            protocol,
            width,
            height,
            ..
        }) => Ok(PreviewData::Image {
            protocol,
            width,
            height,
            format_label: "VIDEO".to_string(),
        }),
        Ok(other) => Ok(other),
        Err(err) => Err(GenerateOutcome::Failed(err)),
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
struct FfprobeMeta {
    format_name: Option<String>,
    duration_secs: Option<String>,
    bit_rate: Option<u64>,
    video_codec: Option<String>,
    width: u32,
    height: u32,
    fps: Option<String>,
    audio_codec: Option<String>,
}

#[derive(Default)]
struct StreamAcc {
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    avg_frame_rate: Option<String>,
}

fn unescape_ffprobe_ini(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                out.push(next);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn parse_fps_ratio(raw: &str) -> Option<String> {
    let (num_str, den_str) = raw.split_once('/')?;
    let num: f64 = num_str.trim().parse().ok()?;
    let den: f64 = den_str.trim().parse().ok()?;
    if den <= 0.0 || num <= 0.0 {
        return None;
    }
    let fps = num / den;
    if !fps.is_finite() {
        return None;
    }
    if (fps - fps.round()).abs() < 0.01 {
        Some(format!("{} fps", fps.round() as u64))
    } else {
        Some(format!("{fps:.2} fps"))
    }
}

fn format_duration_secs(secs_f: f64) -> Option<String> {
    if !secs_f.is_finite() || secs_f < 0.0 {
        return None;
    }
    let total = secs_f.round() as u64;
    let hours = total / 3600;
    let mins = (total % 3600) / 60;
    let secs = total % 60;
    if hours > 0 {
        Some(format!("{hours}:{mins:02}:{secs:02}"))
    } else {
        Some(format!("{mins:02}:{secs:02}"))
    }
}

fn flush_stream(acc: &mut StreamAcc, out: &mut FfprobeMeta) {
    match acc.codec_type.as_deref() {
        Some("video") if out.video_codec.is_none() && out.width == 0 => {
            out.video_codec = acc.codec_name.take();
            out.width = acc.width.unwrap_or(0);
            out.height = acc.height.unwrap_or(0);
            out.fps = acc.avg_frame_rate.as_deref().and_then(parse_fps_ratio);
        }
        Some("audio") if out.audio_codec.is_none() => {
            out.audio_codec = acc.codec_name.take();
        }
        _ => {}
    }
    *acc = StreamAcc::default();
}

fn parse_ffprobe_ini(raw: &str) -> FfprobeMeta {
    enum Section {
        None,
        Stream,
        Format,
    }

    let mut out = FfprobeMeta::default();
    let mut section = Section::None;
    let mut current_stream = StreamAcc::default();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if matches!(section, Section::Stream) {
                flush_stream(&mut current_stream, &mut out);
            }
            let header = &trimmed[1..trimmed.len() - 1];
            section = if header.starts_with("streams.stream.") {
                Section::Stream
            } else if header == "format" {
                Section::Format
            } else {
                Section::None
            };
            continue;
        }

        let Some((k, v)) = trimmed.split_once('=') else {
            continue;
        };
        let key = k.trim();
        let val = unescape_ffprobe_ini(v.trim());
        if val.is_empty() || val == "N/A" {
            continue;
        }

        match section {
            Section::Stream => match key {
                "codec_type" => current_stream.codec_type = Some(val),
                "codec_name" => current_stream.codec_name = Some(val),
                "width" => current_stream.width = val.parse().ok(),
                "height" => current_stream.height = val.parse().ok(),
                "avg_frame_rate" => current_stream.avg_frame_rate = Some(val),
                _ => {}
            },
            Section::Format => match key {
                "format_long_name" => out.format_name = Some(val),
                "duration" => {
                    out.duration_secs = val.parse::<f64>().ok().and_then(format_duration_secs);
                }
                "bit_rate" => out.bit_rate = val.parse().ok(),
                _ => {}
            },
            Section::None => {}
        }
    }

    if matches!(section, Section::Stream) {
        flush_stream(&mut current_stream, &mut out);
    }

    out
}

fn probe_video_metadata(
    path: &Path,
    tools: &VideoToolPaths<'_>,
    is_cancelled: &mut dyn FnMut() -> bool,
) -> Option<Result<FfprobeMeta, ()>> {
    let tmp_path = unique_temp_path(&std::env::temp_dir(), "probe", "ini");
    let out_file = match File::create(&tmp_path) {
        Ok(f) => f,
        Err(_) => return Some(Err(())),
    };
    let _guard = TempFileGuard::new(tmp_path.clone());

    let input = safe_cli_path(path);
    let mut cmd = Command::new(tools.ffprobe);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::from(out_file))
        .stderr(Stdio::null())
        .args([
            "-v",
            "error",
            "-protocol_whitelist",
            "file,pipe",
            "-show_entries",
            "format=duration,bit_rate,format_long_name:stream=codec_type,codec_name,width,height,avg_frame_rate",
            "-of",
            "ini",
        ])
        .arg(&input);

    match run_command_cancellable(&mut cmd, VIDEO_PROBE_TIMEOUT, is_cancelled) {
        SubprocessOutcome::Cancelled => None,
        SubprocessOutcome::Exited(status) if status.success() => {
            match fs::read_to_string(&tmp_path) {
                Ok(raw) => Some(Ok(parse_ffprobe_ini(&raw))),
                Err(_) => Some(Err(())),
            }
        }
        _ => Some(Err(())),
    }
}

fn format_bitrate(bps: u64) -> String {
    if bps >= 1_000_000 {
        format!("{:.1} Mbps", bps as f64 / 1_000_000.0)
    } else if bps >= 1_000 {
        format!("{} kbps", bps / 1_000)
    } else {
        format!("{bps} bps")
    }
}

fn build_video_metadata_fallback(
    entry: &FileEntry,
    note: Option<String>,
    tools: &VideoToolPaths<'_>,
    is_cancelled: &mut dyn FnMut() -> bool,
) -> Option<PreviewData> {
    let probed = match probe_video_metadata(&entry.path, tools, is_cancelled) {
        Some(Ok(meta)) => meta,
        Some(Err(())) => FfprobeMeta::default(),
        None => return None,
    };

    let format_value = probed.format_name.unwrap_or_else(|| {
        if entry.extension.is_empty() {
            "Video".to_string()
        } else {
            entry.extension.to_ascii_uppercase()
        }
    });

    let mut rows = Vec::with_capacity(8);
    rows.push(("Format".to_string(), format_value));

    if probed.width > 0 && probed.height > 0 {
        let res = match probed.fps {
            Some(fps) => format!("{}×{} ({fps})", probed.width, probed.height),
            None => format!("{}×{}", probed.width, probed.height),
        };
        rows.push(("Resolution".to_string(), res));
    }
    if let Some(duration) = probed.duration_secs {
        rows.push(("Duration".to_string(), duration));
    }
    if let Some(vcodec) = probed.video_codec {
        rows.push(("Video".to_string(), vcodec));
    }
    if let Some(acodec) = probed.audio_codec {
        rows.push(("Audio".to_string(), acodec));
    }
    if let Some(bps) = probed.bit_rate {
        rows.push(("Bitrate".to_string(), format_bitrate(bps)));
    }

    let (num, unit) = format_size(entry.size, false);
    rows.push((
        "Size".to_string(),
        format!("{num}{unit} ({} bytes)", entry.size),
    ));

    if entry.modified.is_some() {
        rows.push(("Modified".to_string(), format_time(entry.modified)));
    }

    Some(PreviewData::Metadata {
        title: "Video file".to_string(),
        rows,
        note,
    })
}

/// Builds a video preview using:
/// 1) cached thumbnail (`lsi` disk cache, then FreeDesktop `~/.cache/thumbnails/`),
/// 2) external CLI subprocess (`ffmpegthumbnailer`, then `ffmpeg`),
/// 3) structured video metadata fallback (`ffprobe` + filesystem attributes).
pub fn build_video_preview(
    entry: &FileEntry,
    opts: &PreviewBuildOpts<'_>,
    is_cancelled: &mut dyn FnMut() -> bool,
) -> Option<PreviewData> {
    #[cfg(feature = "image-preview")]
    {
        let base = default_cache_base();
        let lsi_dir = lsi_thumbnail_cache_dir(base.as_deref());
        let fd_dir = freedesktop_thumbnail_dir(base.as_deref());
        build_video_preview_with(
            entry,
            opts,
            lsi_dir.as_deref(),
            fd_dir.as_deref(),
            &VideoToolPaths::default(),
            is_cancelled,
        )
    }
    #[cfg(not(feature = "image-preview"))]
    {
        build_video_preview_with(
            entry,
            opts,
            None,
            None,
            &VideoToolPaths::default(),
            is_cancelled,
        )
    }
}

pub(crate) fn build_video_preview_with(
    entry: &FileEntry,
    opts: &PreviewBuildOpts<'_>,
    lsi_cache_dir: Option<&Path>,
    freedesktop_dir: Option<&Path>,
    tools: &VideoToolPaths<'_>,
    is_cancelled: &mut dyn FnMut() -> bool,
) -> Option<PreviewData> {
    #[cfg(feature = "image-preview")]
    let mut fallback_note: Option<String> = None;
    #[cfg(not(feature = "image-preview"))]
    let fallback_note: Option<String> = None;

    #[cfg(feature = "image-preview")]
    {
        let thumbnails_enabled = opts.config.video_preview
            && opts.config.image_preview
            && opts.config.image_protocol != ImageProtocol::Off;

        if thumbnails_enabled {
            if let Some(picker) = opts.picker {
                // Stage 1: Look for an existing cached thumbnail (lsi cache -> FreeDesktop cache).
                if let Some(cached_path) =
                    find_cached_thumbnail_in(entry, lsi_cache_dir, freedesktop_dir)
                {
                    if let Ok(PreviewData::Image {
                        protocol,
                        width,
                        height,
                        ..
                    }) = decode_image_file_for_pane(&cached_path, picker, opts.pane_size)
                    {
                        return Some(PreviewData::Image {
                            protocol,
                            width,
                            height,
                            format_label: "VIDEO".to_string(),
                        });
                    }
                }

                if is_cancelled() {
                    return None;
                }

                // Stage 2: Extract thumbnail via external CLI (ffmpegthumbnailer -> ffmpeg).
                match generate_and_load_via_cli(
                    entry,
                    lsi_cache_dir,
                    picker,
                    opts.pane_size,
                    tools,
                    is_cancelled,
                ) {
                    Ok(preview) => return Some(preview),
                    Err(GenerateOutcome::Cancelled) => return None,
                    Err(GenerateOutcome::ToolsMissing) => {
                        fallback_note = Some(
                            "Install ffmpegthumbnailer or ffmpeg for video thumbnails".to_string(),
                        );
                    }
                    Err(GenerateOutcome::Failed(reason)) => {
                        fallback_note = Some(format!("Thumbnail unavailable ({reason})"));
                    }
                }
            }
        } else if !opts.config.video_preview {
            fallback_note = Some("Video thumbnails disabled".to_string());
        }
    }

    #[cfg(not(feature = "image-preview"))]
    {
        let _ = (opts, lsi_cache_dir, freedesktop_dir);
    }

    if is_cancelled() {
        return None;
    }

    // Stage 3: Structured video metadata fallback.
    build_video_metadata_fallback(entry, fallback_note, tools, is_cancelled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs_entry::read_directory_with_cache;
    use crate::fs_entry::IdentityCache;
    use crate::preview::PreviewConfig;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lsi_video_test_{tag}_{}_{}",
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

    #[cfg(feature = "image-preview")]
    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = 0xffff_ffff_u32;
        for &b in bytes {
            crc ^= u32::from(b);
            for _ in 0..8 {
                let mask = (crc & 1).wrapping_neg();
                crc = (crc >> 1) ^ (0xedb8_8320 & mask);
            }
        }
        !crc
    }

    #[cfg(feature = "image-preview")]
    fn write_png_with_thumb_attrs(path: &Path, mtime: u64, size: u64) {
        let tmp = path.with_extension("raw.png");
        let img: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
            image::ImageBuffer::from_pixel(24, 16, image::Rgba([40, 120, 210, 255]));
        img.save(&tmp).unwrap();
        let raw = fs::read(&tmp).unwrap();
        let _ = fs::remove_file(&tmp);

        // Insert tEXt chunks right after the 8-byte signature + 25-byte IHDR chunk.
        let split_at = 8 + 4 + 4 + 13 + 4;
        let mut out = Vec::with_capacity(raw.len() + 128);
        out.extend_from_slice(&raw[..split_at]);
        for (k, v) in [
            ("Thumb::MTime", mtime.to_string()),
            ("Thumb::Size", size.to_string()),
        ] {
            let mut payload = Vec::new();
            payload.extend_from_slice(k.as_bytes());
            payload.push(0);
            payload.extend_from_slice(v.as_bytes());

            let mut chunk_body = Vec::with_capacity(4 + payload.len());
            chunk_body.extend_from_slice(b"tEXt");
            chunk_body.extend_from_slice(&payload);
            let crc = crc32(&chunk_body);

            out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
            out.extend_from_slice(&chunk_body);
            out.extend_from_slice(&crc.to_be_bytes());
        }
        out.extend_from_slice(&raw[split_at..]);
        fs::write(path, out).unwrap();
    }

    #[cfg(unix)]
    fn write_exec_script(path: &Path, body: &str) {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        {
            let mut f = File::create(path).unwrap();
            f.write_all(body.as_bytes()).unwrap();
            f.sync_all().unwrap();
        }
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }

    #[test]
    fn resolves_thumbnail_cache_dirs_from_xdg_or_home_fallback() {
        #[cfg(windows)]
        let (home_str, xdg_str) = (r"C:\Users\tester", r"C:\xdg-cache");
        #[cfg(not(windows))]
        let (home_str, xdg_str) = ("/home/tester", "/var/tmp/xdg-cache");

        let home = OsStr::new(home_str);
        let expected_fd = PathBuf::from(home_str).join(".cache").join("thumbnails");
        let expected_lsi = PathBuf::from(home_str)
            .join(".cache")
            .join("lsi")
            .join("thumbnails");

        // Missing $XDG_CACHE_HOME defaults to $HOME/.cache/thumbnails and $HOME/.cache/lsi/thumbnails
        let base_missing = resolve_cache_base(None, Some(home));
        assert_eq!(
            freedesktop_thumbnail_dir(base_missing.as_deref()),
            Some(expected_fd.clone())
        );
        assert_eq!(
            lsi_thumbnail_cache_dir(base_missing.as_deref()),
            Some(expected_lsi.clone())
        );

        // Empty or relative $XDG_CACHE_HOME also falls back to $HOME/.cache/...
        for invalid in [OsStr::new(""), OsStr::new("relative/cache")] {
            let base = resolve_cache_base(Some(invalid), Some(home));
            assert_eq!(
                freedesktop_thumbnail_dir(base.as_deref()),
                Some(expected_fd.clone())
            );
            assert_eq!(
                lsi_thumbnail_cache_dir(base.as_deref()),
                Some(expected_lsi.clone())
            );
        }

        // Valid absolute $XDG_CACHE_HOME overrides $HOME/.cache
        let base_xdg = resolve_cache_base(Some(OsStr::new(xdg_str)), Some(home));
        assert_eq!(
            freedesktop_thumbnail_dir(base_xdg.as_deref()),
            Some(PathBuf::from(xdg_str).join("thumbnails"))
        );
        assert_eq!(
            lsi_thumbnail_cache_dir(base_xdg.as_deref()),
            Some(PathBuf::from(xdg_str).join("lsi").join("thumbnails"))
        );

        // Unset when both environment variables are missing
        assert_eq!(resolve_cache_base(None, None), None);
    }

    #[test]
    fn md5_matches_rfc1321_test_vectors_and_uri_encodes_spaces() {
        assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(md5_hex(b"a"), "0cc175b9c0f1b6a831c399e269772661");
        assert_eq!(md5_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(
            md5_hex(b"message digest"),
            "f96b697d7cb7938d525a2f31aaf161d0"
        );
        assert_eq!(
            md5_hex(b"abcdefghijklmnopqrstuvwxyz"),
            "c3fcd3d76192e4007dfb496cca67e13b"
        );
        assert_eq!(
            md5_hex(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"),
            "d174ab98d277d9f5a5611c2c9f419d9f"
        );
        assert_eq!(
            md5_hex(
                b"12345678901234567890123456789012345678901234567890123456789012345678901234567890"
            ),
            "57edf4a22be3c955ac49da2e2107b67a"
        );

        assert_eq!(
            file_uri_for_path(Path::new("/tmp/My Clip #1.mp4")),
            "file:///tmp/My%20Clip%20%231.mp4"
        );
        assert!(is_video_extension("MP4"));
        assert!(is_video_extension("mkv"));
        assert!(!is_video_extension("png"));
    }

    #[cfg(feature = "image-preview")]
    #[test]
    fn finds_lsi_and_freedesktop_cached_thumbnails_and_rejects_stale_mtime() {
        let root = make_temp_dir("cache");
        let vid_path = root.join("clip.mp4");
        fs::write(&vid_path, b"dummy mp4 payload").unwrap();

        let mut cache = IdentityCache::new();
        let entries = read_directory_with_cache(&root, &mut cache).unwrap();
        let entry = entries
            .iter()
            .find(|e| e.name == "clip.mp4")
            .unwrap()
            .clone();

        let lsi_dir = root.join("lsi_cache");
        let fd_dir = root.join("fd_cache");
        fs::create_dir_all(&lsi_dir).unwrap();
        fs::create_dir_all(fd_dir.join("large")).unwrap();

        let md5 = canonical_uri_md5(&entry.path);
        let fd_thumb = fd_dir.join("large").join(format!("{md5}.png"));

        // Stale MTime must be rejected
        write_png_with_thumb_attrs(&fd_thumb, 1, entry.size);
        assert_eq!(
            find_cached_thumbnail_in(&entry, Some(&lsi_dir), Some(&fd_dir)),
            None
        );

        // Fresh MTime + Size hits FreeDesktop cache without needing any CLI tools
        write_png_with_thumb_attrs(&fd_thumb, entry_mtime_secs(&entry), entry.size);
        assert_eq!(
            find_cached_thumbnail_in(&entry, Some(&lsi_dir), Some(&fd_dir)),
            Some(fd_thumb.clone())
        );

        let picker = crate::preview::create_image_picker(ImageProtocol::Halfblocks, false).unwrap();
        let cfg = PreviewConfig::default();
        let opts = PreviewBuildOpts {
            show_hidden: false,
            is_light_theme: false,
            pane_size: (32, 16),
            config: &cfg,
            picker: Some(&picker),
        };
        let missing_tools = VideoToolPaths {
            ffmpegthumbnailer: "__missing_thumb__",
            ffmpeg: "__missing_ffmpeg__",
            ffprobe: "__missing_ffprobe__",
        };
        let preview = build_video_preview_with(
            &entry,
            &opts,
            Some(&lsi_dir),
            Some(&fd_dir),
            &missing_tools,
            &mut || false,
        );
        match preview {
            Some(PreviewData::Image {
                width,
                height,
                format_label,
                ..
            }) => {
                assert_eq!((width, height), (24, 16));
                assert_eq!(format_label, "VIDEO");
            }
            other => panic!("expected Image from FreeDesktop cache, got {other:?}"),
        }

        // lsi's own disk cache takes precedence over FreeDesktop cache
        let lsi_thumb = lsi_dir.join(format!("{}.jpg", lsi_cache_stem(&md5, &entry)));
        let img: image::ImageBuffer<image::Rgb<u8>, Vec<u8>> =
            image::ImageBuffer::from_pixel(12, 8, image::Rgb([10, 200, 30]));
        img.save(&lsi_thumb).unwrap();
        assert_eq!(
            find_cached_thumbnail_in(&entry, Some(&lsi_dir), Some(&fd_dir)),
            Some(lsi_thumb)
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(all(unix, feature = "image-preview"))]
    #[test]
    fn uses_external_cli_subprocesses_with_fallback_order_and_populates_cache() {
        let root = make_temp_dir("cli_mock");
        let vid_path = root.join("-dash_video.mkv");
        fs::write(&vid_path, b"video bytes").unwrap();

        let sample_jpg = root.join("sample.jpg");
        let img: image::ImageBuffer<image::Rgb<u8>, Vec<u8>> =
            image::ImageBuffer::from_pixel(30, 20, image::Rgb([220, 80, 40]));
        img.save(&sample_jpg).unwrap();

        let mut cache = IdentityCache::new();
        let entries = read_directory_with_cache(&root, &mut cache).unwrap();
        let entry = entries
            .iter()
            .find(|e| e.name == "-dash_video.mkv")
            .unwrap()
            .clone();

        let picker = crate::preview::create_image_picker(ImageProtocol::Halfblocks, false).unwrap();
        let cfg = PreviewConfig::default();
        let opts = PreviewBuildOpts {
            show_hidden: false,
            is_light_theme: false,
            pane_size: (32, 16),
            config: &cfg,
            picker: Some(&picker),
        };

        // 1. ffmpegthumbnailer runs first and writes disk cache
        let lsi_dir_1 = root.join("cache1");
        let mock_thumb = root.join("mock_thumb.sh");
        write_exec_script(
            &mock_thumb,
            &format!(
                "#!/bin/sh\nout=\"\"\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = \"-o\" ]; then out=\"$2\"; shift 2; continue; fi\n  shift\ndone\ncp \"{}\" \"$out\"\n",
                sample_jpg.display()
            ),
        );
        let tools1 = VideoToolPaths {
            ffmpegthumbnailer: mock_thumb.to_str().unwrap(),
            ffmpeg: "__missing_ffmpeg__",
            ffprobe: "__missing_ffprobe__",
        };
        let res1 =
            build_video_preview_with(&entry, &opts, Some(&lsi_dir_1), None, &tools1, &mut || {
                false
            });
        assert!(matches!(
            res1,
            Some(PreviewData::Image {
                width: 30,
                height: 20,
                ..
            })
        ));
        assert!(find_cached_thumbnail_in(&entry, Some(&lsi_dir_1), None).is_some());

        // 2. When ffmpegthumbnailer is missing, ffmpeg tries 00:00:05, retries 00:00:00, and caches
        let lsi_dir_2 = root.join("cache2");
        let mock_ffmpeg = root.join("mock_ffmpeg.sh");
        write_exec_script(
            &mock_ffmpeg,
            &format!(
                "#!/bin/sh\nseek=\"\"\nlast=\"\"\nprev=\"\"\nfor arg in \"$@\"; do\n  if [ \"$prev\" = \"-ss\" ]; then seek=\"$arg\"; fi\n  prev=\"$arg\"\n  last=\"$arg\"\ndone\nif [ \"$seek\" = \"00:00:05\" ]; then exit 1; fi\ncp \"{}\" \"$last\"\n",
                sample_jpg.display()
            ),
        );
        let tools2 = VideoToolPaths {
            ffmpegthumbnailer: "__missing_thumb__",
            ffmpeg: mock_ffmpeg.to_str().unwrap(),
            ffprobe: "__missing_ffprobe__",
        };
        let res2 =
            build_video_preview_with(&entry, &opts, Some(&lsi_dir_2), None, &tools2, &mut || {
                false
            });
        assert!(matches!(
            res2,
            Some(PreviewData::Image {
                width: 30,
                height: 20,
                ..
            })
        ));
        assert!(find_cached_thumbnail_in(&entry, Some(&lsi_dir_2), None).is_some());

        // 3. Cancellation terminates slow subprocess cleanly and returns None
        let slow_thumb = root.join("slow_thumb.sh");
        write_exec_script(&slow_thumb, "#!/bin/sh\nsleep 2\n");
        let tools3 = VideoToolPaths {
            ffmpegthumbnailer: slow_thumb.to_str().unwrap(),
            ffmpeg: "__missing_ffmpeg__",
            ffprobe: "__missing_ffprobe__",
        };
        let lsi_dir_3 = root.join("cache3");
        let mut checks = 0_u32;
        let cancelled =
            build_video_preview_with(&entry, &opts, Some(&lsi_dir_3), None, &tools3, &mut || {
                checks += 1;
                checks >= 2
            });
        assert!(cancelled.is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn falls_back_to_ffprobe_and_basic_video_metadata_when_tools_missing() {
        let sample_ini = "\
# ffprobe output

[streams.stream.0]
codec_name=h264
codec_type=video
width=1920
height=1080
avg_frame_rate=30000/1001

[streams.stream.1]
codec_name=aac
codec_type=audio
avg_frame_rate=0/0

[format]
format_long_name=QuickTime / MOV
duration=125.400000
bit_rate=4200000
";
        let parsed = parse_ffprobe_ini(sample_ini);
        assert_eq!(parsed.format_name.as_deref(), Some("QuickTime / MOV"));
        assert_eq!(parsed.duration_secs.as_deref(), Some("02:05"));
        assert_eq!(parsed.bit_rate, Some(4_200_000));
        assert_eq!(parsed.video_codec.as_deref(), Some("h264"));
        assert_eq!((parsed.width, parsed.height), (1920, 1080));
        assert_eq!(parsed.fps.as_deref(), Some("29.97 fps"));
        assert_eq!(parsed.audio_codec.as_deref(), Some("aac"));

        let root = make_temp_dir("meta_fallback");
        let vid_path = root.join("movie.mp4");
        fs::write(&vid_path, vec![0_u8; 2048]).unwrap();

        let mut cache = IdentityCache::new();
        let entries = read_directory_with_cache(&root, &mut cache).unwrap();
        let entry = entries
            .iter()
            .find(|e| e.name == "movie.mp4")
            .unwrap()
            .clone();

        let picker = crate::preview::create_image_picker(ImageProtocol::Halfblocks, false);
        let cfg = PreviewConfig::default();
        let opts = PreviewBuildOpts {
            show_hidden: false,
            is_light_theme: false,
            pane_size: (40, 16),
            config: &cfg,
            picker: picker.as_ref(),
        };

        let no_tools = VideoToolPaths {
            ffmpegthumbnailer: "__missing_thumb__",
            ffmpeg: "__missing_ffmpeg__",
            ffprobe: "__missing_ffprobe__",
        };
        let fallback = build_video_preview_with(
            &entry,
            &opts,
            Some(&root.join("empty_cache")),
            None,
            &no_tools,
            &mut || false,
        );
        match fallback {
            Some(PreviewData::Metadata { title, rows, note }) => {
                assert_eq!(title, "Video file");
                assert!(rows.iter().any(|(k, v)| k == "Format" && v == "MP4"));
                assert!(rows.iter().any(|(k, _)| k == "Size"));
                #[cfg(feature = "image-preview")]
                assert!(note
                    .unwrap_or_default()
                    .contains("ffmpegthumbnailer or ffmpeg"));
                #[cfg(not(feature = "image-preview"))]
                let _ = note;
            }
            other => panic!("expected Metadata fallback, got {other:?}"),
        }

        #[cfg(unix)]
        {
            let mock_probe = root.join("mock_ffprobe.sh");
            write_exec_script(
                &mock_probe,
                "#!/bin/sh\ncat << 'EOF'\n[streams.stream.0]\ncodec_name=vp9\ncodec_type=video\nwidth=1280\nheight=720\navg_frame_rate=60/1\n[format]\nformat_long_name=Matroska / WebM\nduration=65.0\nbit_rate=1500000\nEOF\n",
            );
            let probe_tools = VideoToolPaths {
                ffmpegthumbnailer: "__missing_thumb__",
                ffmpeg: "__missing_ffmpeg__",
                ffprobe: mock_probe.to_str().unwrap(),
            };
            match build_video_preview_with(
                &entry,
                &opts,
                Some(&root.join("empty_cache")),
                None,
                &probe_tools,
                &mut || false,
            ) {
                Some(PreviewData::Metadata { rows, .. }) => {
                    assert!(rows
                        .iter()
                        .any(|(k, v)| k == "Resolution" && v == "1280×720 (60 fps)"));
                    assert!(rows.iter().any(|(k, v)| k == "Duration" && v == "01:05"));
                    assert!(rows.iter().any(|(k, v)| k == "Video" && v == "vp9"));
                    assert!(rows.iter().any(|(k, v)| k == "Bitrate" && v == "1.5 Mbps"));
                }
                other => panic!("expected enriched Metadata fallback, got {other:?}"),
            }
        }

        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(feature = "image-preview")]
    #[test]
    fn real_ffmpeg_generates_thumbnail_for_synthetic_short_mp4() {
        let probe = Command::new("ffmpeg")
            .arg("-version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if !matches!(probe, Ok(s) if s.success()) {
            return;
        }

        let root = make_temp_dir("real_ffmpeg");
        let vid_path = root.join("short.mp4");
        let status = Command::new("ffmpeg")
            .args([
                "-nostdin",
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=duration=1:size=64x48:rate=10",
                "-pix_fmt",
                "yuv420p",
                "-c:v",
                "mpeg4",
            ])
            .arg(&vid_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(status.success());

        let mut cache = IdentityCache::new();
        let entries = read_directory_with_cache(&root, &mut cache).unwrap();
        let entry = entries
            .iter()
            .find(|e| e.name == "short.mp4")
            .unwrap()
            .clone();

        let picker = crate::preview::create_image_picker(ImageProtocol::Halfblocks, false).unwrap();
        let cfg = PreviewConfig::default();
        let opts = PreviewBuildOpts {
            show_hidden: false,
            is_light_theme: false,
            pane_size: (40, 18),
            config: &cfg,
            picker: Some(&picker),
        };
        let lsi_cache = root.join("lsi_cache");
        let ff_only = VideoToolPaths {
            ffmpegthumbnailer: "__missing_thumb__",
            ffmpeg: "ffmpeg",
            ffprobe: "ffprobe",
        };

        let first =
            build_video_preview_with(&entry, &opts, Some(&lsi_cache), None, &ff_only, &mut || {
                false
            });
        match first {
            Some(PreviewData::Image {
                width,
                height,
                format_label,
                ..
            }) => {
                assert_eq!((width, height), (64, 48));
                assert_eq!(format_label, "VIDEO");
            }
            other => panic!("expected real ffmpeg Image preview, got {other:?}"),
        }

        // Second call must hit the on-disk cache without spawning ffmpeg at all.
        let no_tools = VideoToolPaths {
            ffmpegthumbnailer: "__missing_thumb__",
            ffmpeg: "__missing_ffmpeg__",
            ffprobe: "__missing_ffprobe__",
        };
        let second = build_video_preview_with(
            &entry,
            &opts,
            Some(&lsi_cache),
            None,
            &no_tools,
            &mut || false,
        );
        assert!(matches!(
            second,
            Some(PreviewData::Image {
                width: 64,
                height: 48,
                ..
            })
        ));

        let _ = fs::remove_dir_all(&root);
    }
}
