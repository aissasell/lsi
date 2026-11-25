# Changelog

All notable changes to `lsi` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.2] - 2026-11-23

### Added

- **Real file previews (`src/preview.rs`)**:
  - **Syntax highlighting**: Pure-Rust `syntect` (`default-fancy`) highlighting with lazy `OnceLock` initialization, extension/filename/shebang detection, active light/dark theme matching, and a `1000`-char line-length guard to avoid regex backtracking stalls on minified files.
  - **Terminal image previews**: `ratatui-image` + pruned pure-Rust `image` (`png`, `jpeg`, `gif`, `webp`, `bmp`, `ico`) supporting Kitty Graphics Protocol, iTerm2 inline images, Sixel, and Unicode half-blocks, with lazy stdio protocol probing, fast `Triangle` downscaling, `20 MiB` / `100 MP` decode limits, and Kitty overlay cleanup in `Viewport::Inline`.
  - **Asynchronous preview worker & LRU cache**: Dedicated `std::thread` + `mpsc` worker with request coalescing, `35 ms` image debounce, bounded 24-entry `lru::LruCache`, and conditional `event::poll` while loading (`0%` idle CPU).
  - **Preview scrolling & config/feature controls**: Scroll text/directory/metadata previews with `J` / `Alt+Down` and `K` / `Alt+Up` (`preview_scroll_down` / `preview_scroll_up` in `keybinds.toml`), configure `syntax_highlight`, `image_preview`, `video_preview`, `image_protocol`, and `preview_max_file_mb` in `config.toml`, and optionally disable default features via Cargo `--no-default-features` (`syntax-highlight`, `image-preview`).
- **Video thumbnail previews (`src/video_preview.rs`)**:
  - Checks cached thumbnails first (`$XDG_CACHE_HOME/lsi/thumbnails/` and FreeDesktop.org `$XDG_CACHE_HOME/thumbnails/`, falling back to `$HOME/.cache/lsi/thumbnails/` and `$HOME/.cache/thumbnails/` when `$XDG_CACHE_HOME` is unset, empty, or relative, with PNG `Thumb::MTime`/`Thumb::Size` freshness validation).
  - Falls back to external CLI extraction (`ffmpegthumbnailer`, then `ffmpeg` with keyframe input seeking, short-clip `0s` retry, active worker cancellation, and bounded disk caching).
  - Falls back to structured video metadata (`ffprobe` stream info + size/modified details) when CLI tools are unavailable or `video_preview = false`.

### Changed

- Removed the `Preview: <filename>` border title above the side-by-side preview pane.

## [1.2.0] - 2026-11-20

### Added

- **Custom keybindings (`$HOME/.config/lsi/keybinds.toml`)**: Added full TOML customization for all 51 actions across Normal, Search, Confirm Delete, Create Item, Rename Item, and Help modes (supporting single keys, arrays, modifier combos, and `[]` unbinding), with footer hints and the `?` help modal dynamically displaying active keybinds.

### Changed

- Moved custom themes from `$HOME/.config/lsi/theme.toml` to named `.toml` files under `$HOME/.config/lsi/themes/<name>.toml` selected via `theme = "<name>"` in `config.toml` or `--theme <name>`. Files matching built-in theme names are ignored, omitted color keys inherit from `eza-dark`, and missing custom files fall back to `eza-dark`.

## [1.1.0] - 2026-11-19

### Added

- **Multi-selection & file manipulation**:
  - `Space` toggles multi-selection on the highlighted entry (and advances one row), while `Esc` cancels multi-selection before clearing filters or exiting.
  - `y` yanks (copies), `x` cuts, and `p` pastes files and directories into the current directory (handling collisions and cross-device moves).
  - `Delete` moves selected items to the FreeDesktop.org Trash bin (`files/` + `info/*.trashinfo`).
  - `Shift+Delete` permanently deletes selected items after an inline `(y/N)` confirmation prompt.
- **New item creation (`a`)**: Reassigned `a` from toggling hidden files (kept on `.`) to an inline creation prompt (`NEW`) — enter a relative name and press `Enter` to create a file, or end the name with `/` (e.g., `newdir/`) to create a folder; press `Esc` to cancel.
- **Inline rename (`r`)**: Added interactive file/folder renaming on `r` (`RENAME`), prefilling the footer input with the highlighted item's full name (including extension) with cursor movement (`Left`/`Right`/`Home`/`End`/`Backspace`/`Delete`/`Ctrl+u`), `Enter` to confirm, and `Esc` to cancel.

### Changed

- Reassigned toggling the permissions column from `p` to `P` (`Shift+p`) so `p` can be used for paste.
- Reassigned reverse sort order from `r` to `R` (`Shift+r`) and moved manual directory refresh to `F5` / `Ctrl+r` so `r` can be used for renaming.

## [1.0.0] - 2026-11-19

### Added

- **User configuration (`$HOME/.config/lsi/config.toml`)**: Added TOML configuration loading with priority `CLI flags > config.toml > theme.toml > defaults`, supporting `show_borders`, `show_footer`, `show_hidden`, `long`, `show_permissions`, `show_ownership`, `show_time`, `show_size`, `show_icons`, `show_preview`, `dirs_first`, `reverse`, `sort`, `theme`, and `max_rows`.
- **Custom theme (`$HOME/.config/lsi/theme.toml`)**: Added custom palette loading (`--theme custom`) with support for hex (`#rrggbb` / `rrggbb`), `[r, g, b]` arrays, `rgb(r, g, b)` strings, and named terminal colors. When `theme.toml` exists, `custom` automatically becomes the default theme while preserving all 7 built-in themes in the `T` / `c` cycle.
- **Border and footer toggles**: Added `--borders` / `--no-borders`, `--footer` / `--no-footer`, and runtime `B` / `F` keybindings (dynamically adjusting inline viewport height).
- **Positive CLI overrides**: Added `--icons` and `--dirs-first` flags so CLI arguments can override `config.toml` values in either direction.
- **Shell integration (`--cwd-file <PATH>`)**:
  - Pressing `q` exits and writes `current_dir` to `--cwd-file` for Fish, Bash, and Zsh `cd`-on-exit wrappers.
  - Pressing `Q`, `Ctrl+c`, or `Esc` (when no search filter is active) exits without modifying the shell's working directory.
- **Documentation & CI**: Added the Astro Starlight documentation site in `docs/`, the `lsi` terminal-prompt-and-table SVG logo and favicon, and the initial GitHub Actions workflow.

## [0.2.0] - 2026-11-17

### Changed

- **Inline layout alignment**: Removed the top header bar and extra header row allocation so the directory table starts directly below the shell prompt.
- **In-terminal file opening**: Pressing `Enter`, `l`, or `Right` on a text file now opens it directly inside the current terminal via `$VISUAL` / `$EDITOR` instead of spawning an external terminal via `xdg-open`; `O` explicitly launches the system desktop opener (`xdg-open`).

### Added

- Initial `README.md` with installation, CLI options, and keybinding tables.

## [0.1.0] - 2026-11-17

### Added

- Initial release of `lsi`, an interactive `eza`-style file explorer built with Rust, `ratatui`, and `crossterm`.
- **Inline viewport with dynamic sizing**: Renders inline below the prompt (`Viewport::Inline`) and grows dynamically up to `-n` / `--max-rows` (default `20`), including a compact layout when height is `< 6`.
- **`eza`-inspired columns & Nerd Fonts**: Colorized Unix permissions (`-p`), human-readable sizes (`-s` / `--no-size`), UID/GID-resolved owner and group (`-o`), modification timestamps (`-t`), hidden file toggling (`-a`), and Nerd Font icons for folders, filenames, and extensions (`--no-icons`).
- **Navigation, preview & filtering**: Directory tree traversal (`h`/`j`/`k`/`l`, arrow keys, mouse scroll), back/forward navigation history (`-` / `+`), side-by-side preview pane (`v` / `Tab`), live `/` substring filtering, sorting (`--sort`, `-r`, `--no-dirs-first`), help modal (`?`), and 7 built-in color themes (`eza-dark`, `catppuccin`, `dracula`, `nord`, `gruvbox`, `tokyo-night`, `solarized-light`).

[1.2.2]: https://github.com/aissasell/lsi/compare/1.2...1.2.2
[1.2.0]: https://github.com/aissasell/lsi/compare/1.1...1.2
[1.1.0]: https://github.com/aissasell/lsi/compare/1.0.0...1.1
[1.0.0]: https://github.com/aissasell/lsi/compare/0.2.0...1.0.0
[0.2.0]: https://github.com/aissasell/lsi/compare/3de8ef3...0.2.0
[0.1.0]: https://github.com/aissasell/lsi/commit/3de8ef3
