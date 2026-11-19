---
title: Changelog
description: Release history and notable changes for lsi.
---

All notable changes to `lsi` are documented on this page.

## [Unreleased]

### Added

- **User configuration (`$HOME/.config/lsi/config.toml`)**: Added TOML configuration loading with priority `CLI flags > config.toml > theme.toml > defaults`, supporting `show_borders`, `show_footer`, `show_hidden`, `long`, `show_permissions`, `show_ownership`, `show_time`, `show_size`, `show_icons`, `show_preview`, `dirs_first`, `reverse`, `sort`, `theme`, and `max_rows`.
- **Custom theme (`$HOME/.config/lsi/theme.toml`)**: Added custom palette loading (`--theme custom`) with support for hex (`#rrggbb` / `rrggbb`), `[r, g, b]` arrays, `rgb(r, g, b)` strings, and named terminal colors. When `theme.toml` exists, `custom` automatically becomes the default theme while preserving all 7 built-in themes in the `T` / `c` cycle.
- **Border and footer toggles**: Added `--borders` / `--no-borders`, `--footer` / `--no-footer`, and runtime `B` / `F` keybindings (dynamically adjusting inline viewport height).
- **Positive CLI overrides**: Added `--icons` and `--dirs-first` flags so CLI arguments can override `config.toml` values in either direction.
- **Shell integration (`--cwd-file <PATH>`)**:
  - Pressing `q` exits and writes `current_dir` to `--cwd-file` for Fish, Bash, and Zsh `cd`-on-exit wrappers.
  - Pressing `Q`, `Ctrl+c`, or `Esc` (when no search filter is active) exits without modifying the shell's working directory.
- **Documentation & icon**: Added the Astro Starlight documentation site in `docs/` and the `lsi` terminal-prompt-and-table SVG logo and favicon.

## [0.2.0] — 2026-11-17

### Changed

- **Inline layout alignment**: Removed the top header bar and extra header row allocation so the directory table starts directly below the shell prompt.
- **In-terminal file opening**: Pressing `Enter`, `l`, or `Right` on a text file now opens it directly inside the current terminal via `$VISUAL` / `$EDITOR` instead of spawning an external terminal via `xdg-open`; `O` explicitly launches the system desktop opener (`xdg-open`).

### Added

- Initial `README.md` with installation, CLI options, and keybinding tables.

## [0.1.0] — 2026-11-17

### Added

- Initial release of `lsi`, an interactive `eza`-style file explorer built with Rust, `ratatui`, and `crossterm`.
- **Inline viewport with dynamic sizing**: Renders inline below the prompt (`Viewport::Inline`) and grows dynamically up to `-n` / `--max-rows` (default `20`), including a compact layout when height is `< 6`.
- **`eza`-inspired columns & Nerd Fonts**: Colorized Unix permissions (`-p`), human-readable sizes (`-s` / `--no-size`), UID/GID-resolved owner and group (`-o`), modification timestamps (`-t`), hidden file toggling (`-a`), and Nerd Font icons for folders, filenames, and extensions (`--no-icons`).
- **Navigation, preview & filtering**: Directory tree traversal (`h`/`j`/`k`/`l`, arrow keys, mouse scroll), back/forward navigation history (`-` / `+`), side-by-side preview pane (`v` / `Tab`), live `/` substring filtering, sorting (`--sort`, `-r`, `--no-dirs-first`), help modal (`?`), and 7 built-in color themes (`eza-dark`, `catppuccin`, `dracula`, `nord`, `gruvbox`, `tokyo-night`, `solarized-light`).
