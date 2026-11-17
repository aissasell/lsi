use crate::fs_entry::FileEntry;

/// Visual category used to color an icon and its corresponding entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconCategory {
    Directory,
    Symlink,
    BrokenSymlink,
    Executable,
    Code,
    Config,
    Document,
    Archive,
    Media,
    Regular,
}

/// Nerd Font icon glyph and semantic category for a filesystem entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconInfo {
    pub glyph: &'static str,
    pub category: IconCategory,
}

/// Returns the Nerd Font icon (or empty glyph if `show_icons` is false) and category for `entry`.
pub fn get_icon(entry: &FileEntry, show_icons: bool) -> IconInfo {
    let info = resolve_icon(entry);
    if show_icons {
        info
    } else {
        IconInfo {
            glyph: "",
            category: info.category,
        }
    }
}

fn resolve_icon(entry: &FileEntry) -> IconInfo {
    if entry.is_dir {
        if entry.is_symlink {
            return IconInfo {
                glyph: "\u{f482}",
                category: IconCategory::Symlink,
            };
        }
        let glyph = match entry.name.to_ascii_lowercase().as_str() {
            ".git" => "\u{e5fb}",
            ".github" => "\u{e5fd}",
            "node_modules" => "\u{e5fa}",
            ".config" | "config" | "etc" => "\u{e5fc}",
            "src" | "lib" => "\u{e5ff}",
            "target" | "build" | "dist" | "bin" => "\u{f013}",
            "docs" | "doc" => "\u{f02d}",
            "tests" | "test" => "\u{f0c3}",
            "images" | "assets" | "public" | "media" => "\u{f03e}",
            "downloads" => "\u{f019}",
            _ => "\u{f07b}",
        };
        return IconInfo {
            glyph,
            category: IconCategory::Directory,
        };
    }

    if entry.symlink_broken {
        return IconInfo {
            glyph: "\u{f0c1}",
            category: IconCategory::BrokenSymlink,
        };
    }

    if entry.is_symlink {
        return IconInfo {
            glyph: "\u{f0c1}",
            category: IconCategory::Symlink,
        };
    }

    if let Some(exact) = match_exact_filename(&entry.name) {
        return exact;
    }

    if let Some(ext_icon) = match_extension(&entry.extension) {
        return ext_icon;
    }

    if entry.is_executable {
        return IconInfo {
            glyph: "\u{f471}",
            category: IconCategory::Executable,
        };
    }

    IconInfo {
        glyph: "\u{f15b}",
        category: IconCategory::Regular,
    }
}

fn match_exact_filename(name: &str) -> Option<IconInfo> {
    let lower = name.to_ascii_lowercase();
    let info = match lower.as_str() {
        "cargo.toml" => IconInfo {
            glyph: "\u{e7a8}",
            category: IconCategory::Code,
        },
        "cargo.lock" => IconInfo {
            glyph: "\u{e7a8}",
            category: IconCategory::Config,
        },
        "package.json" | "package-lock.json" | ".nvmrc" => IconInfo {
            glyph: "\u{e71e}",
            category: IconCategory::Config,
        },
        "dockerfile"
        | "docker-compose.yml"
        | "docker-compose.yaml"
        | "compose.yml"
        | "compose.yaml"
        | ".dockerignore" => IconInfo {
            glyph: "\u{f308}",
            category: IconCategory::Config,
        },
        "makefile" | "cmakelists.txt" | "justfile" => IconInfo {
            glyph: "\u{e779}",
            category: IconCategory::Config,
        },
        ".gitignore" | ".gitattributes" | ".gitmodules" => IconInfo {
            glyph: "\u{e702}",
            category: IconCategory::Config,
        },
        "license" | "license.md" | "license.txt" | "copying" => IconInfo {
            glyph: "\u{f02d}",
            category: IconCategory::Document,
        },
        "readme" | "readme.md" | "readme.txt" | "changelog.md" => IconInfo {
            glyph: "\u{f48a}",
            category: IconCategory::Document,
        },
        ".env" | ".env.local" | ".env.example" => IconInfo {
            glyph: "\u{f462}",
            category: IconCategory::Config,
        },
        _ => return None,
    };
    Some(info)
}

fn match_extension(ext: &str) -> Option<IconInfo> {
    let info = match ext {
        // Code
        "rs" => IconInfo {
            glyph: "\u{e7a8}",
            category: IconCategory::Code,
        },
        "py" | "pyi" | "ipynb" => IconInfo {
            glyph: "\u{e73c}",
            category: IconCategory::Code,
        },
        "js" | "mjs" | "cjs" => IconInfo {
            glyph: "\u{e74e}",
            category: IconCategory::Code,
        },
        "ts" => IconInfo {
            glyph: "\u{e628}",
            category: IconCategory::Code,
        },
        "tsx" | "jsx" => IconInfo {
            glyph: "\u{e7ba}",
            category: IconCategory::Code,
        },
        "go" => IconInfo {
            glyph: "\u{e627}",
            category: IconCategory::Code,
        },
        "c" | "h" => IconInfo {
            glyph: "\u{e61e}",
            category: IconCategory::Code,
        },
        "cpp" | "cc" | "cxx" | "hpp" => IconInfo {
            glyph: "\u{e61d}",
            category: IconCategory::Code,
        },
        "java" | "jar" | "class" => IconInfo {
            glyph: "\u{e738}",
            category: IconCategory::Code,
        },
        "kt" | "kts" => IconInfo {
            glyph: "\u{e634}",
            category: IconCategory::Code,
        },
        "zig" => IconInfo {
            glyph: "\u{e6a9}",
            category: IconCategory::Code,
        },
        "lua" => IconInfo {
            glyph: "\u{e620}",
            category: IconCategory::Code,
        },
        "rb" => IconInfo {
            glyph: "\u{e739}",
            category: IconCategory::Code,
        },
        "php" => IconInfo {
            glyph: "\u{e73d}",
            category: IconCategory::Code,
        },
        "cs" => IconInfo {
            glyph: "\u{f031b}",
            category: IconCategory::Code,
        },
        "swift" => IconInfo {
            glyph: "\u{e755}",
            category: IconCategory::Code,
        },
        "html" | "htm" => IconInfo {
            glyph: "\u{e736}",
            category: IconCategory::Code,
        },
        "css" | "scss" | "sass" => IconInfo {
            glyph: "\u{e749}",
            category: IconCategory::Code,
        },
        "vue" => IconInfo {
            glyph: "\u{e6a0}",
            category: IconCategory::Code,
        },
        "svelte" => IconInfo {
            glyph: "\u{e697}",
            category: IconCategory::Code,
        },
        "sh" | "bash" | "zsh" | "fish" => IconInfo {
            glyph: "\u{e795}",
            category: IconCategory::Code,
        },

        // Data & Config
        "json" | "jsonc" | "jsonl" => IconInfo {
            glyph: "\u{e60b}",
            category: IconCategory::Config,
        },
        "toml" | "yaml" | "yml" | "ini" | "conf" | "cfg" => IconInfo {
            glyph: "\u{e615}",
            category: IconCategory::Config,
        },
        "xml" | "plist" => IconInfo {
            glyph: "\u{f05c0}",
            category: IconCategory::Config,
        },
        "sql" | "db" | "sqlite" | "sqlite3" => IconInfo {
            glyph: "\u{f1c0}",
            category: IconCategory::Config,
        },
        "csv" | "tsv" => IconInfo {
            glyph: "\u{f1c3}",
            category: IconCategory::Config,
        },
        "lock" => IconInfo {
            glyph: "\u{f023}",
            category: IconCategory::Config,
        },

        // Documents
        "md" | "markdown" | "mdx" => IconInfo {
            glyph: "\u{e73e}",
            category: IconCategory::Document,
        },
        "txt" | "log" | "rst" | "org" => IconInfo {
            glyph: "\u{f15c}",
            category: IconCategory::Document,
        },
        "pdf" => IconInfo {
            glyph: "\u{f1c1}",
            category: IconCategory::Document,
        },
        "doc" | "docx" => IconInfo {
            glyph: "\u{f1c2}",
            category: IconCategory::Document,
        },
        "xls" | "xlsx" => IconInfo {
            glyph: "\u{f1c3}",
            category: IconCategory::Document,
        },
        "ppt" | "pptx" => IconInfo {
            glyph: "\u{f1c4}",
            category: IconCategory::Document,
        },

        // Archives
        "zip" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "zst" | "7z" | "rar" | "deb" | "rpm"
        | "iso" => IconInfo {
            glyph: "\u{f410}",
            category: IconCategory::Archive,
        },

        // Media
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "ico" | "bmp" => IconInfo {
            glyph: "\u{f1c5}",
            category: IconCategory::Media,
        },
        "mp3" | "wav" | "flac" | "ogg" | "m4a" => IconInfo {
            glyph: "\u{f001}",
            category: IconCategory::Media,
        },
        "mp4" | "mkv" | "webm" | "avi" | "mov" => IconInfo {
            glyph: "\u{f03d}",
            category: IconCategory::Media,
        },
        "ttf" | "otf" | "woff" | "woff2" => IconInfo {
            glyph: "\u{f031}",
            category: IconCategory::Media,
        },

        _ => return None,
    };
    Some(info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn dummy_entry(name: &str, is_dir: bool, is_exec: bool) -> FileEntry {
        let extension = if is_dir {
            String::new()
        } else {
            std::path::Path::new(name)
                .extension()
                .and_then(|e| e.to_str())
                .map(str::to_ascii_lowercase)
                .unwrap_or_default()
        };
        FileEntry {
            name: name.to_string(),
            path: PathBuf::from(name),
            is_dir,
            is_symlink: false,
            symlink_target: None,
            symlink_broken: false,
            is_executable: is_exec,
            is_hidden: name.starts_with('.'),
            size: 128,
            mode: 0o644,
            uid: 1000,
            gid: 1000,
            owner: "user".to_string(),
            group: "group".to_string(),
            modified: None,
            extension,
        }
    }

    #[test]
    fn special_directory_icons() {
        let git = dummy_entry(".git", true, false);
        assert_eq!(get_icon(&git, true).glyph, "\u{e5fb}");
        assert_eq!(get_icon(&git, true).category, IconCategory::Directory);

        let src = dummy_entry("src", true, false);
        assert_eq!(get_icon(&src, true).glyph, "\u{e5ff}");

        let regular_dir = dummy_entry("projects", true, false);
        assert_eq!(get_icon(&regular_dir, true).glyph, "\u{f07b}");
    }

    #[test]
    fn exact_filename_and_extension_icons() {
        let cargo = dummy_entry("Cargo.toml", false, false);
        assert_eq!(get_icon(&cargo, true).glyph, "\u{e7a8}");

        let rs = dummy_entry("main.rs", false, false);
        assert_eq!(get_icon(&rs, true).glyph, "\u{e7a8}");
        assert_eq!(get_icon(&rs, true).category, IconCategory::Code);

        let archive = dummy_entry("bundle.tar.gz", false, false);
        assert_eq!(get_icon(&archive, true).glyph, "\u{f410}");
        assert_eq!(get_icon(&archive, true).category, IconCategory::Archive);

        let md = dummy_entry("README.md", false, false);
        assert_eq!(get_icon(&md, true).glyph, "\u{f48a}");
        assert_eq!(get_icon(&md, true).category, IconCategory::Document);
    }

    #[test]
    fn no_icons_clears_glyph_only() {
        let rs = dummy_entry("lib.rs", false, false);
        let info = get_icon(&rs, false);
        assert_eq!(info.glyph, "");
        assert_eq!(info.category, IconCategory::Code);
    }
}
