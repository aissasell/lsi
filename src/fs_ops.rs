use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::Local;

/// Returns a collision-free destination path inside `dest_dir` for `file_name`.
pub fn unique_destination(dest_dir: &Path, file_name: &OsStr) -> PathBuf {
    let direct = dest_dir.join(file_name);
    if fs::symlink_metadata(&direct).is_err() {
        return direct;
    }

    let name = file_name.to_string_lossy();
    let (stem, ext) = match name.rfind('.').filter(|&idx| idx > 0) {
        Some(idx) => (&name[..idx], &name[idx..]),
        None => (name.as_ref(), ""),
    };

    let mut counter = 1_usize;
    loop {
        let candidate_name = if counter == 1 {
            format!("{stem}_copy{ext}")
        } else {
            format!("{stem}_copy_{counter}{ext}")
        };
        let candidate = dest_dir.join(candidate_name);
        if fs::symlink_metadata(&candidate).is_err() {
            return candidate;
        }
        counter += 1;
    }
}

/// Copies a file, symlink, or directory recursively into `dest_dir`.
pub fn copy_entry(src: &Path, dest_dir: &Path) -> Result<PathBuf> {
    let src_meta = fs::symlink_metadata(src)
        .with_context(|| format!("Cannot inspect '{}'", src.display()))?;

    if src_meta.file_type().is_dir() {
        let canon_src = fs::canonicalize(src)?;
        let canon_dest_dir = fs::canonicalize(dest_dir)?;
        if canon_dest_dir.starts_with(&canon_src) {
            bail!(
                "Cannot copy directory '{}' into itself or a subdirectory",
                src.display()
            );
        }
    }

    let file_name = src.file_name().context("Source path has no file name")?;
    let dest = unique_destination(dest_dir, file_name);
    copy_path_recursive(src, &dest)?;
    Ok(dest)
}

/// Moves a file, symlink, or directory into `dest_dir`.
///
/// Returns `Ok(None)` when `src` is already located directly inside `dest_dir`.
pub fn move_entry(src: &Path, dest_dir: &Path) -> Result<Option<PathBuf>> {
    let src_meta = fs::symlink_metadata(src)
        .with_context(|| format!("Cannot inspect '{}'", src.display()))?;

    if let Some(parent) = src.parent() {
        if let (Ok(canon_parent), Ok(canon_dest_dir)) =
            (fs::canonicalize(parent), fs::canonicalize(dest_dir))
        {
            if canon_parent == canon_dest_dir {
                return Ok(None);
            }
        }
    }

    if src_meta.file_type().is_dir() {
        let canon_src = fs::canonicalize(src)?;
        let canon_dest_dir = fs::canonicalize(dest_dir)?;
        if canon_dest_dir.starts_with(&canon_src) {
            bail!(
                "Cannot move directory '{}' into itself or a subdirectory",
                src.display()
            );
        }
    }

    let file_name = src.file_name().context("Source path has no file name")?;
    let dest = unique_destination(dest_dir, file_name);
    rename_or_cross_device_move(src, &dest)?;
    Ok(Some(dest))
}

/// Permanently deletes a file, symlink (without following it), or directory tree.
pub fn permanent_delete(path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path)
        .with_context(|| format!("Cannot inspect '{}'", path.display()))?;
    let file_type = meta.file_type();
    if file_type.is_symlink() {
        #[cfg(windows)]
        let res = if fs::metadata(path).is_ok_and(|target| target.is_dir()) {
            fs::remove_dir(path)
        } else {
            fs::remove_file(path)
        };
        #[cfg(not(windows))]
        let res = fs::remove_file(path);
        res.with_context(|| format!("Failed to remove '{}'", path.display()))?;
    } else if file_type.is_dir() {
        fs::remove_dir_all(path)
            .with_context(|| format!("Failed to remove directory '{}'", path.display()))?;
    } else {
        fs::remove_file(path)
            .with_context(|| format!("Failed to remove '{}'", path.display()))?;
    }
    Ok(())
}

/// Moves `path` to the user's FreeDesktop.org Trash bin.
pub fn move_to_trash(path: &Path) -> Result<PathBuf> {
    let trash_dir = default_trash_dir()?;
    move_to_trash_in(path, &trash_dir)
}

/// Moves `path` to the specified FreeDesktop.org `trash_dir` (`files/` + `info/*.trashinfo`).
pub fn move_to_trash_in(path: &Path, trash_dir: &Path) -> Result<PathBuf> {
    let files_dir = trash_dir.join("files");
    let info_dir = trash_dir.join("info");
    fs::create_dir_all(&files_dir)
        .with_context(|| format!("Failed to create '{}'", files_dir.display()))?;
    fs::create_dir_all(&info_dir)
        .with_context(|| format!("Failed to create '{}'", info_dir.display()))?;

    let abs_parent = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => {
            fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
        }
        _ => std::env::current_dir()?,
    };
    let file_name = path.file_name().context("Path has no file name")?;
    let orig_abs = abs_parent.join(file_name);
    let canon_trash = fs::canonicalize(trash_dir).unwrap_or_else(|_| trash_dir.to_path_buf());
    if orig_abs == canon_trash || canon_trash.starts_with(&orig_abs) {
        bail!("Cannot move the Trash directory into itself");
    }

    let base_name = file_name.to_string_lossy();
    let (stem, ext) = match base_name.rfind('.').filter(|&idx| idx > 0) {
        Some(idx) => (&base_name[..idx], &base_name[idx..]),
        None => (base_name.as_ref(), ""),
    };

    let (trashed_file, info_path) = {
        let mut counter = 0_usize;
        loop {
            let cand_name = if counter == 0 {
                base_name.to_string()
            } else {
                format!("{stem}.{counter}{ext}")
            };
            let cand_file = files_dir.join(&cand_name);
            let cand_info = info_dir.join(format!("{cand_name}.trashinfo"));

            if fs::symlink_metadata(&cand_file).is_err() {
                match OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&cand_info)
                {
                    Ok(mut info_file) => {
                        let encoded_path = percent_encode_path(&orig_abs);
                        let deletion_date = Local::now().format("%Y-%m-%dT%H:%M:%S");
                        let contents = format!(
                            "[Trash Info]\nPath={encoded_path}\nDeletionDate={deletion_date}\n"
                        );
                        if let Err(err) = info_file.write_all(contents.as_bytes()) {
                            let _ = fs::remove_file(&cand_info);
                            return Err(err).with_context(|| {
                                format!("Failed to write '{}'", cand_info.display())
                            });
                        }
                        break (cand_file, cand_info);
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(err) => {
                        return Err(err)
                            .with_context(|| format!("Failed to create '{}'", cand_info.display()))
                    }
                }
            }
            counter += 1;
        }
    };

    if let Err(err) = rename_or_cross_device_move(path, &trashed_file) {
        let _ = fs::remove_file(&info_path);
        return Err(err);
    }

    Ok(trashed_file)
}

/// Creates a new file or directory inside `parent_dir`.
///
/// Names ending with `/` create a directory (and any missing parents);
/// all other relative names create an empty file (creating parent directories
/// if needed). Rejects empty names, parent traversal (`..`), absolute paths,
/// and paths that already exist.
pub fn create_entry(parent_dir: &Path, raw_input: &str) -> Result<PathBuf> {
    let trimmed = raw_input.trim();
    if trimmed.is_empty() {
        bail!("Name cannot be empty");
    }

    let is_dir = trimmed.ends_with('/');
    let clean_rel = trimmed.trim_end_matches('/');
    if clean_rel.is_empty() {
        bail!("Invalid path: {trimmed}");
    }

    let mut rel_path = PathBuf::new();
    for component in Path::new(clean_rel).components() {
        match component {
            Component::Normal(part) => rel_path.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!("Invalid path: {trimmed}");
            }
        }
    }

    if rel_path.as_os_str().is_empty() {
        bail!("Name cannot be empty");
    }

    let target = parent_dir.join(&rel_path);
    if target.symlink_metadata().is_ok() {
        bail!("Already exists: {}", target.display());
    }

    if is_dir {
        fs::create_dir_all(&target)
            .with_context(|| format!("Failed to create directory {}", target.display()))?;
    } else {
        if let Some(parent) = target.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create parent directory for {}", target.display())
                })?;
            }
        }
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .with_context(|| format!("Failed to create file {}", target.display()))?;
    }

    Ok(target)
}

/// Renames a file, directory, or symlink within its parent directory.
///
/// Rejects empty names, path separators (`/`), `.` / `..`, and collisions with
/// an existing sibling entry (preventing silent overwrites on Unix).
pub fn rename_entry(src: &Path, raw_new_name: &str) -> Result<PathBuf> {
    let trimmed = raw_new_name.trim();
    if trimmed.is_empty() {
        bail!("Name cannot be empty");
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed == "." || trimmed == ".." {
        bail!("Invalid name: {trimmed}");
    }

    let mut components = Path::new(trimmed).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => {}
        _ => bail!("Invalid name: {trimmed}"),
    }

    let parent = src
        .parent()
        .with_context(|| format!("Cannot determine parent of {}", src.display()))?;
    let dst = parent.join(trimmed);

    if dst == src {
        return Ok(dst);
    }
    if dst.symlink_metadata().is_ok() {
        bail!("Already exists: {}", dst.display());
    }

    fs::rename(src, &dst)
        .with_context(|| format!("Failed to rename {} to {}", src.display(), dst.display()))?;
    Ok(dst)
}

fn default_trash_dir() -> Result<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME").filter(|s| !s.is_empty()) {
        return Ok(PathBuf::from(xdg).join("Trash"));
    }
    if let Some(home) = std::env::var_os("HOME").filter(|s| !s.is_empty()) {
        return Ok(PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("Trash"));
    }
    #[cfg(windows)]
    if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA").filter(|s| !s.is_empty()) {
        return Ok(PathBuf::from(local_app_data).join("lsi").join("Trash"));
    }
    if let Some(profile) = std::env::var_os("USERPROFILE").filter(|s| !s.is_empty()) {
        return Ok(PathBuf::from(profile)
            .join(".local")
            .join("share")
            .join("Trash"));
    }
    bail!("Cannot locate Trash directory (HOME and XDG_DATA_HOME are unset)")
}

fn create_symlink(target: &Path, dest: &Path, target_is_dir: bool) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let _ = target_is_dir;
        std::os::unix::fs::symlink(target, dest)
    }
    #[cfg(windows)]
    {
        if target_is_dir {
            std::os::windows::fs::symlink_dir(target, dest)
        } else {
            std::os::windows::fs::symlink_file(target, dest)
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (target, dest, target_is_dir);
        Err(std::io::Error::new(
            ErrorKind::Unsupported,
            "Symlinks are unsupported on this platform",
        ))
    }
}

fn copy_path_recursive(src: &Path, dest: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(src)
        .with_context(|| format!("Cannot inspect '{}'", src.display()))?;
    let file_type = meta.file_type();

    if file_type.is_symlink() {
        let target = fs::read_link(src)
            .with_context(|| format!("Cannot read symlink '{}'", src.display()))?;
        let target_is_dir = fs::metadata(src).is_ok_and(|m| m.is_dir());
        create_symlink(&target, dest, target_is_dir).with_context(|| {
            format!(
                "Failed to copy symlink '{}' -> '{}'",
                src.display(),
                dest.display()
            )
        })?;
        return Ok(());
    }

    if file_type.is_dir() {
        fs::create_dir(dest)
            .with_context(|| format!("Failed to create directory '{}'", dest.display()))?;
        for entry in
            fs::read_dir(src).with_context(|| format!("Cannot read '{}'", src.display()))?
        {
            let entry = entry?;
            let child_src = entry.path();
            let child_dest = dest.join(entry.file_name());
            copy_path_recursive(&child_src, &child_dest)?;
        }
        // Apply directory permissions after copying children so read-only dirs copy cleanly.
        let _ = fs::set_permissions(dest, meta.permissions());
        return Ok(());
    }

    fs::copy(src, dest).with_context(|| {
        format!(
            "Failed to copy '{}' to '{}'",
            src.display(),
            dest.display()
        )
    })?;
    Ok(())
}

fn is_cross_device_error(err: &std::io::Error) -> bool {
    if err.kind() == ErrorKind::CrossesDevices {
        return true;
    }
    #[cfg(unix)]
    if err.raw_os_error() == Some(libc::EXDEV) {
        return true;
    }
    #[cfg(windows)]
    if err.raw_os_error() == Some(17) {
        return true;
    }
    false
}

fn rename_or_cross_device_move(src: &Path, dest: &Path) -> Result<()> {
    match fs::rename(src, dest) {
        Ok(()) => Ok(()),
        Err(err) if is_cross_device_error(&err) => {
            copy_path_recursive(src, dest)?;
            permanent_delete(src)?;
            Ok(())
        }
        Err(err) => Err(err).with_context(|| {
            format!(
                "Failed to move '{}' to '{}'",
                src.display(),
                dest.display()
            )
        }),
    }
}

fn percent_encode_path(path: &Path) -> String {
    let raw_bytes: std::borrow::Cow<'_, [u8]> = {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            std::borrow::Cow::Borrowed(path.as_os_str().as_bytes())
        }
        #[cfg(not(unix))]
        {
            std::borrow::Cow::Owned(path.to_string_lossy().into_owned().into_bytes())
        }
    };
    let mut out = String::with_capacity(raw_bytes.len());
    for &b in raw_bytes.as_ref() {
        if b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            use std::fmt::Write;
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lsi_fsops_test_{tag}_{}_{}",
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
    fn unique_destination_handles_collisions_and_dotfiles() {
        let dir = make_temp_dir("unique");
        fs::write(dir.join("notes.txt"), "1").unwrap();
        fs::write(dir.join(".env"), "2").unwrap();
        fs::create_dir(dir.join("folder")).unwrap();

        assert_eq!(
            unique_destination(&dir, OsStr::new("fresh.txt")),
            dir.join("fresh.txt")
        );
        let first_copy = unique_destination(&dir, OsStr::new("notes.txt"));
        assert_eq!(first_copy, dir.join("notes_copy.txt"));
        fs::write(&first_copy, "c1").unwrap();
        assert_eq!(
            unique_destination(&dir, OsStr::new("notes.txt")),
            dir.join("notes_copy_2.txt")
        );

        assert_eq!(
            unique_destination(&dir, OsStr::new(".env")),
            dir.join(".env_copy")
        );
        assert_eq!(
            unique_destination(&dir, OsStr::new("folder")),
            dir.join("folder_copy")
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn copies_files_dirs_symlinks_and_guards_self_copy() {
        let root = make_temp_dir("copy");
        let src_dir = root.join("src_dir");
        let sub = src_dir.join("nested");
        let dest = root.join("dest");
        fs::create_dir_all(&sub).unwrap();
        fs::create_dir_all(&dest).unwrap();
        fs::write(src_dir.join("a.txt"), "hello").unwrap();
        fs::write(sub.join("b.txt"), "world").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("a.txt", src_dir.join("link_a")).unwrap();

        // Copy directory recursively including symlink
        let copied_dir = copy_entry(&src_dir, &dest).unwrap();
        assert_eq!(copied_dir, dest.join("src_dir"));
        assert_eq!(fs::read_to_string(copied_dir.join("a.txt")).unwrap(), "hello");
        assert_eq!(
            fs::read_to_string(copied_dir.join("nested/b.txt")).unwrap(),
            "world"
        );
        #[cfg(unix)]
        {
            let link_meta = fs::symlink_metadata(copied_dir.join("link_a")).unwrap();
            assert!(link_meta.file_type().is_symlink());
        }

        // Self-copy guard
        assert!(copy_entry(&src_dir, &sub).is_err());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn moves_and_permanently_deletes_without_following_dir_symlinks() {
        let root = make_temp_dir("move_del");
        let target_dir = root.join("important_dir");
        let work_dir = root.join("work");
        let dest_dir = root.join("dest");
        fs::create_dir_all(&target_dir).unwrap();
        fs::create_dir_all(&work_dir).unwrap();
        fs::create_dir_all(&dest_dir).unwrap();
        fs::write(target_dir.join("keep.txt"), "safe").unwrap();
        fs::write(work_dir.join("item.txt"), "move me").unwrap();

        // Moving into same directory is a no-op
        assert_eq!(move_entry(&work_dir.join("item.txt"), &work_dir).unwrap(), None);

        // Moving into another directory moves the file
        let moved = move_entry(&work_dir.join("item.txt"), &dest_dir)
            .unwrap()
            .unwrap();
        assert_eq!(moved, dest_dir.join("item.txt"));
        assert!(!work_dir.join("item.txt").exists());
        assert_eq!(fs::read_to_string(&moved).unwrap(), "move me");

        // Deleting a symlink to a directory deletes only the symlink, preserving target_dir
        #[cfg(unix)]
        {
            let link_to_dir = work_dir.join("dir_link");
            std::os::unix::fs::symlink(&target_dir, &link_to_dir).unwrap();
            permanent_delete(&link_to_dir).unwrap();
            assert!(fs::symlink_metadata(&link_to_dir).is_err());
            assert!(target_dir.join("keep.txt").exists());
        }

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn moves_to_trash_and_writes_valid_trashinfo() {
        let root = make_temp_dir("trash");
        let trash_dir = root.join("Trash");
        let file = root.join("my file.txt");
        fs::write(&file, "trash contents").unwrap();

        let trashed = move_to_trash_in(&file, &trash_dir).unwrap();
        assert!(!file.exists());
        assert_eq!(trashed, trash_dir.join("files/my file.txt"));
        assert_eq!(fs::read_to_string(&trashed).unwrap(), "trash contents");

        let info = fs::read_to_string(trash_dir.join("info/my file.txt.trashinfo")).unwrap();
        assert!(info.starts_with("[Trash Info]\nPath="));
        assert!(info.contains("my%20file.txt\n"));
        assert!(info.contains("DeletionDate="));

        // Second file with the same name gets disambiguated
        let file2 = root.join("my file.txt");
        fs::write(&file2, "second").unwrap();
        let trashed2 = move_to_trash_in(&file2, &trash_dir).unwrap();
        assert_eq!(trashed2, trash_dir.join("files/my file.1.txt"));
        assert!(trash_dir.join("info/my file.1.txt.trashinfo").is_file());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn create_entry_creates_files_folders_and_nested_paths() {
        let root = make_temp_dir("create-entry");

        let file = create_entry(&root, "notes.txt").unwrap();
        assert_eq!(file, root.join("notes.txt"));
        assert!(file.is_file());

        let dir = create_entry(&root, "newdir/").unwrap();
        assert_eq!(dir, root.join("newdir"));
        assert!(dir.is_dir());

        let nested_file = create_entry(&root, "nested/sub/item.txt").unwrap();
        assert_eq!(nested_file, root.join("nested/sub/item.txt"));
        assert!(nested_file.is_file());

        let nested_dir = create_entry(&root, "nested/deep/dir/").unwrap();
        assert_eq!(nested_dir, root.join("nested/deep/dir"));
        assert!(nested_dir.is_dir());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn create_entry_rejects_collisions_and_invalid_paths() {
        let root = make_temp_dir("create-entry-errors");
        fs::write(root.join("existing.txt"), "hi").unwrap();
        fs::create_dir(root.join("existing_dir")).unwrap();

        assert!(create_entry(&root, "existing.txt").is_err());
        assert!(create_entry(&root, "existing_dir/").is_err());
        assert!(create_entry(&root, "").is_err());
        assert!(create_entry(&root, "   ").is_err());
        assert!(create_entry(&root, "/").is_err());
        assert!(create_entry(&root, "/abs/path").is_err());
        assert!(create_entry(&root, "../escape").is_err());
        assert!(create_entry(&root, "sub/../escape").is_err());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rename_entry_renames_files_extensions_dirs_and_symlinks() {
        let root = make_temp_dir("rename-entry");
        let file = root.join("notes.txt");
        fs::write(&file, "hello").unwrap();

        let renamed_file = rename_entry(&file, "notes.md").unwrap();
        assert_eq!(renamed_file, root.join("notes.md"));
        assert!(!file.exists());
        assert_eq!(fs::read_to_string(&renamed_file).unwrap(), "hello");

        let plain = root.join("plain");
        fs::write(&plain, "plain-data").unwrap();
        let renamed_plain = rename_entry(&plain, "plain.log").unwrap();
        assert_eq!(renamed_plain, root.join("plain.log"));
        assert!(!plain.exists());

        let dir_a = root.join("dir_a");
        fs::create_dir(&dir_a).unwrap();
        fs::write(dir_a.join("inside.txt"), "inside").unwrap();
        let dir_b = rename_entry(&dir_a, "dir_b").unwrap();
        assert_eq!(dir_b, root.join("dir_b"));
        assert!(!dir_a.exists());
        assert_eq!(fs::read_to_string(dir_b.join("inside.txt")).unwrap(), "inside");

        #[cfg(unix)]
        {
            let link = root.join("link_a");
            std::os::unix::fs::symlink(dir_b.join("inside.txt"), &link).unwrap();
            let link_renamed = rename_entry(&link, "link_b").unwrap();
            assert_eq!(link_renamed, root.join("link_b"));
            assert!(link.symlink_metadata().is_err());
            assert!(link_renamed
                .symlink_metadata()
                .unwrap()
                .file_type()
                .is_symlink());
            assert_eq!(fs::read_to_string(dir_b.join("inside.txt")).unwrap(), "inside");
        }

        let same = rename_entry(&renamed_file, "notes.md").unwrap();
        assert_eq!(same, renamed_file);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rename_entry_rejects_collisions_and_invalid_names() {
        let root = make_temp_dir("rename-entry-errors");
        let a = root.join("a.txt");
        let b = root.join("b.txt");
        fs::write(&a, "alpha").unwrap();
        fs::write(&b, "beta").unwrap();

        assert!(rename_entry(&a, "b.txt").is_err());
        assert_eq!(fs::read_to_string(&a).unwrap(), "alpha");
        assert_eq!(fs::read_to_string(&b).unwrap(), "beta");

        for invalid in ["", "   ", ".", "..", "sub/name.txt", "folder/", "/abs"] {
            assert!(
                rename_entry(&a, invalid).is_err(),
                "expected error for invalid rename {invalid:?}"
            );
        }

        let _ = fs::remove_dir_all(&root);
    }
}
