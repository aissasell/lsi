use std::collections::HashMap;
#[cfg(unix)]
use std::ffi::CStr;
use std::fs;
use std::io::ErrorKind;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result};
use chrono::{DateTime, Local};

/// Metadata and classification for a single filesystem entry.
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub symlink_target: Option<PathBuf>,
    pub symlink_broken: bool,
    pub is_executable: bool,
    pub is_hidden: bool,
    pub size: u64,
    pub mode: u32,
    pub uid: u32,
    #[allow(dead_code)]
    pub gid: u32,
    pub owner: String,
    pub group: String,
    pub modified: Option<SystemTime>,
    pub extension: String,
}

/// Caches UID -> username and GID -> group name lookups across directory reads.
#[derive(Debug, Default)]
pub struct IdentityCache {
    users: HashMap<u32, String>,
    groups: HashMap<u32, String>,
}

impl IdentityCache {
    /// Creates an empty identity cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolves a numeric UID to a username, caching the result.
    pub fn resolve_user(&mut self, uid: u32) -> String {
        if let Some(name) = self.users.get(&uid) {
            return name.clone();
        }
        let name = lookup_username(uid).unwrap_or_else(|| uid.to_string());
        self.users.insert(uid, name.clone());
        name
    }

    /// Resolves a numeric GID to a group name, caching the result.
    pub fn resolve_group(&mut self, gid: u32) -> String {
        if let Some(name) = self.groups.get(&gid) {
            return name.clone();
        }
        let name = lookup_groupname(gid).unwrap_or_else(|| gid.to_string());
        self.groups.insert(gid, name.clone());
        name
    }
}

/// Returns the effective UID of the current process (`0` on non-Unix platforms).
#[cfg(unix)]
pub fn current_uid() -> u32 {
    // SAFETY: `geteuid` has no preconditions and always succeeds.
    unsafe { libc::geteuid() }
}

/// Returns `0` on non-Unix platforms.
#[cfg(not(unix))]
pub fn current_uid() -> u32 {
    0
}

#[cfg(unix)]
fn lookup_username(uid: u32) -> Option<String> {
    let mut pwd = std::mem::MaybeUninit::<libc::passwd>::uninit();
    let mut buf = vec![0_u8; 1024];
    let mut result: *mut libc::passwd = std::ptr::null_mut();

    loop {
        // SAFETY: `pwd`, `buf`, and `result` point to valid initialized/allocated memory.
        let rc = unsafe {
            libc::getpwuid_r(
                uid,
                pwd.as_mut_ptr(),
                buf.as_mut_ptr().cast::<libc::c_char>(),
                buf.len(),
                &mut result,
            )
        };
        if rc == libc::ERANGE {
            if buf.len() >= 65_536 {
                return None;
            }
            buf.resize(buf.len() * 2, 0);
            continue;
        }
        if rc != 0 || result.is_null() {
            return None;
        }
        // SAFETY: `getpwuid_r` succeeded with non-null `result` pointing into `pwd` and `buf`.
        let name_ptr = unsafe { (*result).pw_name };
        if name_ptr.is_null() {
            return None;
        }
        // SAFETY: `pw_name` is a valid NUL-terminated C string inside `buf`.
        let c_str = unsafe { CStr::from_ptr(name_ptr) };
        return Some(c_str.to_string_lossy().into_owned());
    }
}

#[cfg(not(unix))]
fn lookup_username(_uid: u32) -> Option<String> {
    std::env::var("USERNAME")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| Some("-".to_string()))
}

#[cfg(unix)]
fn lookup_groupname(gid: u32) -> Option<String> {
    let mut grp = std::mem::MaybeUninit::<libc::group>::uninit();
    let mut buf = vec![0_u8; 1024];
    let mut result: *mut libc::group = std::ptr::null_mut();

    loop {
        // SAFETY: `grp`, `buf`, and `result` point to valid initialized/allocated memory.
        let rc = unsafe {
            libc::getgrgid_r(
                gid,
                grp.as_mut_ptr(),
                buf.as_mut_ptr().cast::<libc::c_char>(),
                buf.len(),
                &mut result,
            )
        };
        if rc == libc::ERANGE {
            if buf.len() >= 65_536 {
                return None;
            }
            buf.resize(buf.len() * 2, 0);
            continue;
        }
        if rc != 0 || result.is_null() {
            return None;
        }
        // SAFETY: `getgrgid_r` succeeded with non-null `result` pointing into `grp` and `buf`.
        let name_ptr = unsafe { (*result).gr_name };
        if name_ptr.is_null() {
            return None;
        }
        // SAFETY: `gr_name` is a valid NUL-terminated C string inside `buf`.
        let c_str = unsafe { CStr::from_ptr(name_ptr) };
        return Some(c_str.to_string_lossy().into_owned());
    }
}

#[cfg(not(unix))]
fn lookup_groupname(_gid: u32) -> Option<String> {
    Some("-".to_string())
}

/// Reads all entries in `dir`, resolving metadata and ownership with a temporary cache.
#[allow(dead_code)]
pub fn read_directory(dir: &Path) -> Result<Vec<FileEntry>> {
    let mut cache = IdentityCache::new();
    read_directory_with_cache(dir, &mut cache)
}

/// Reads all entries in `dir` using a shared `IdentityCache`.
pub fn read_directory_with_cache(dir: &Path, cache: &mut IdentityCache) -> Result<Vec<FileEntry>> {
    let read_dir = fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory '{}'", dir.display()))?;

    let mut entries = Vec::new();
    for dir_entry in read_dir {
        let Ok(dir_entry) = dir_entry else {
            continue;
        };
        let path = dir_entry.path();
        let name = dir_entry.file_name().to_string_lossy().into_owned();
        if let Some(entry) = build_file_entry(name, path, cache) {
            entries.push(entry);
        }
    }

    Ok(entries)
}

#[cfg(unix)]
fn platform_entry_attrs(
    link_meta: &fs::Metadata,
    target_meta: Option<&fs::Metadata>,
    is_dir: bool,
    is_symlink: bool,
    name: &str,
    _ext: &str,
) -> (u32, u32, u32, bool, bool) {
    let mode = link_meta.mode();
    let effective_exec_mode = target_meta.map(|m| m.mode()).unwrap_or(mode);
    let uid = link_meta.uid();
    let gid = link_meta.gid();
    let is_executable = !is_dir && !is_symlink && (effective_exec_mode & 0o111 != 0);
    let is_hidden = name.starts_with('.');
    (mode, uid, gid, is_executable, is_hidden)
}

#[cfg(not(unix))]
fn platform_entry_attrs(
    link_meta: &fs::Metadata,
    target_meta: Option<&fs::Metadata>,
    is_dir: bool,
    is_symlink: bool,
    name: &str,
    ext: &str,
) -> (u32, u32, u32, bool, bool) {
    let is_executable = !is_dir
        && !is_symlink
        && matches!(
            ext.to_ascii_lowercase().as_str(),
            "exe" | "bat" | "cmd" | "com" | "ps1"
        );
    let readonly = target_meta.unwrap_or(link_meta).permissions().readonly();
    let mode = if is_symlink {
        0o120_777
    } else if is_dir {
        if readonly {
            0o040_555
        } else {
            0o040_755
        }
    } else if readonly {
        if is_executable {
            0o100_555
        } else {
            0o100_444
        }
    } else if is_executable {
        0o100_755
    } else {
        0o100_644
    };
    #[cfg(windows)]
    let is_hidden = name.starts_with('.') || (link_meta.file_attributes() & 0x2 != 0);
    #[cfg(not(windows))]
    let is_hidden = name.starts_with('.');
    (mode, 0, 0, is_executable, is_hidden)
}

fn build_file_entry(name: String, path: PathBuf, cache: &mut IdentityCache) -> Option<FileEntry> {
    let link_meta = fs::symlink_metadata(&path).ok()?;
    let file_type = link_meta.file_type();
    let is_symlink = file_type.is_symlink();

    let (is_dir, symlink_broken, target_meta) = if is_symlink {
        match fs::metadata(&path) {
            Ok(target_meta) => (target_meta.is_dir(), false, Some(target_meta)),
            Err(err) => {
                let broken = err.kind() != ErrorKind::PermissionDenied;
                (false, broken, None)
            }
        }
    } else {
        (link_meta.is_dir(), false, None)
    };

    let symlink_target = if is_symlink {
        fs::read_link(&path).ok()
    } else {
        None
    };

    let extension = if is_dir {
        String::new()
    } else {
        Path::new(&name)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default()
    };

    let (mode, uid, gid, is_executable, is_hidden) = platform_entry_attrs(
        &link_meta,
        target_meta.as_ref(),
        is_dir,
        is_symlink,
        &name,
        &extension,
    );
    let owner = cache.resolve_user(uid);
    let group = cache.resolve_group(gid);

    Some(FileEntry {
        name,
        path,
        is_dir,
        is_symlink,
        symlink_target,
        symlink_broken,
        is_executable,
        is_hidden,
        size: link_meta.len(),
        mode,
        uid,
        gid,
        owner,
        group,
        modified: link_meta.modified().ok(),
        extension,
    })
}

/// Formats a standard 10-character Unix permission string (`drwxr-xr-x`, `lrwxrwxrwx`, etc.).
pub fn format_permissions(mode: u32, is_dir: bool, is_symlink: bool) -> String {
    let file_char = if is_symlink {
        'l'
    } else if is_dir {
        'd'
    } else {
        match mode & 0o170_000 {
            0o140_000 => 's',
            0o060_000 => 'b',
            0o020_000 => 'c',
            0o010_000 => 'p',
            0o040_000 => 'd',
            0o120_000 => 'l',
            _ => '-',
        }
    };

    let ur = if mode & 0o400 != 0 { 'r' } else { '-' };
    let uw = if mode & 0o200 != 0 { 'w' } else { '-' };
    let ux = match (mode & 0o100 != 0, mode & 0o4000 != 0) {
        (true, true) => 's',
        (false, true) => 'S',
        (true, false) => 'x',
        (false, false) => '-',
    };

    let gr = if mode & 0o040 != 0 { 'r' } else { '-' };
    let gw = if mode & 0o020 != 0 { 'w' } else { '-' };
    let gx = match (mode & 0o010 != 0, mode & 0o2000 != 0) {
        (true, true) => 's',
        (false, true) => 'S',
        (true, false) => 'x',
        (false, false) => '-',
    };

    let or = if mode & 0o004 != 0 { 'r' } else { '-' };
    let ow = if mode & 0o002 != 0 { 'w' } else { '-' };
    let ox = match (mode & 0o001 != 0, mode & 0o1000 != 0) {
        (true, true) => 't',
        (false, true) => 'T',
        (true, false) => 'x',
        (false, false) => '-',
    };

    let mut out = String::with_capacity(10);
    out.push(file_char);
    out.push(ur);
    out.push(uw);
    out.push(ux);
    out.push(gr);
    out.push(gw);
    out.push(gx);
    out.push(or);
    out.push(ow);
    out.push(ox);
    out
}

/// Formats file size in eza-style `(number, unit)` pairs, returning `("-", "")` for directories.
pub fn format_size(bytes: u64, is_dir: bool) -> (String, &'static str) {
    if is_dir {
        return ("-".to_string(), "");
    }

    const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];
    if bytes < 1024 {
        return (bytes.to_string(), UNITS[0]);
    }

    let mut value = bytes as f64;
    let mut unit_idx = 0;
    while value >= 1024.0 && unit_idx < UNITS.len() - 1 {
        value /= 1024.0;
        unit_idx += 1;
    }

    let num = if value >= 10.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    };
    (num, UNITS[unit_idx])
}

/// Formats a modification timestamp as `YYYY-MM-DD HH:MM`.
pub fn format_time(time: Option<SystemTime>) -> String {
    let Some(sys_time) = time else {
        return "-".to_string();
    };
    let local: DateTime<Local> = sys_time.into();
    local.format("%Y-%m-%d %H:%M").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::UNIX_EPOCH;

    #[test]
    fn formats_permissions_correctly() {
        assert_eq!(format_permissions(0o040_755, true, false), "drwxr-xr-x");
        assert_eq!(format_permissions(0o100_644, false, false), "-rw-r--r--");
        assert_eq!(format_permissions(0o120_777, false, true), "lrwxrwxrwx");
        assert_eq!(format_permissions(0o104_755, false, false), "-rwsr-xr-x");
        assert_eq!(format_permissions(0o102_640, false, false), "-rw-r-S---");
        assert_eq!(format_permissions(0o041_777, true, false), "drwxrwxrwt");
        assert_eq!(format_permissions(0o041_776, true, false), "drwxrwxrwT");
    }

    #[test]
    fn formats_sizes_correctly() {
        assert_eq!(format_size(4096, true), ("-".to_string(), ""));
        assert_eq!(format_size(0, false), ("0".to_string(), "B"));
        assert_eq!(format_size(512, false), ("512".to_string(), "B"));
        assert_eq!(format_size(1536, false), ("1.5".to_string(), "K"));
        assert_eq!(format_size(12 * 1024, false), ("12".to_string(), "K"));
        assert_eq!(
            format_size(3 * 1024 * 1024, false),
            ("3.0".to_string(), "M")
        );
    }

    #[test]
    fn formats_time_cleanly() {
        assert_eq!(format_time(None), "-");
        let formatted = format_time(Some(UNIX_EPOCH));
        assert_eq!(formatted.len(), 16);
        assert!(formatted.contains('-'));
        assert!(formatted.contains(':'));
    }

    #[test]
    fn reads_temp_directory_and_resolves_symlink() {
        let base = std::env::temp_dir().join(format!("lsi_fs_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("sub")).unwrap();
        fs::write(base.join("hello.rs"), "fn main() {}\n").unwrap();
        fs::write(base.join(".hidden"), "secret").unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(base.join("sub"), base.join("sub_link")).unwrap();
            std::os::unix::fs::symlink(base.join("missing"), base.join("broken_link")).unwrap();
        }

        let entries = read_directory(&base).unwrap();
        #[cfg(unix)]
        assert_eq!(entries.len(), 5);
        #[cfg(not(unix))]
        assert_eq!(entries.len(), 3);

        let hidden = entries.iter().find(|e| e.name == ".hidden").unwrap();
        assert!(hidden.is_hidden);
        assert!(!hidden.is_dir);

        #[cfg(unix)]
        {
            let sub_link = entries.iter().find(|e| e.name == "sub_link").unwrap();
            assert!(sub_link.is_symlink);
            assert!(sub_link.is_dir);
            assert!(!sub_link.symlink_broken);

            let broken = entries.iter().find(|e| e.name == "broken_link").unwrap();
            assert!(broken.is_symlink);
            assert!(broken.symlink_broken);
        }

        let rs_file = entries.iter().find(|e| e.name == "hello.rs").unwrap();
        assert_eq!(rs_file.extension, "rs");
        assert!(!rs_file.owner.is_empty());
        assert!(!rs_file.group.is_empty());

        let _ = fs::remove_dir_all(&base);
    }
}
