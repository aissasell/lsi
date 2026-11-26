<div align="center">
  <img src="assets/logo.svg" alt="lsi logo" width="96" />
  <h1><code>lsi</code> — Interactive <code>eza</code>-style Terminal File Explorer</h1>
  <p>
    <a href="https://aissasell.github.io/lsi/">Documentation</a> •
    <a href="https://aissasell.github.io/lsi/guides/getting-started/">Getting Started</a> •
    <a href="https://github.com/aissasell/lsi/releases/latest">Releases</a>
  </p>
</div>

`lsi` is an interactive directory listing tool written in Rust for Linux, macOS, and Windows. Inspired by `eza`, it renders inline directly below your shell prompt with Nerd Font icons, metadata columns, file operations, and a live preview pane.

https://github.com/user-attachments/assets/68f05969-d8f0-4a57-b392-d1c0fecd0b5d

## Highlights

- **Inline viewport**: Renders below your prompt without taking over the alternate screen, dynamically scaling up to `--max-rows`.
- **`eza`-style metadata & Nerd Font icons**: Colorized permissions, human-readable sizes, ownership, timestamps, and semantic file/folder icons.
- **Fast navigation & live search**: Step through directories, jump back/forward in history, sort, and filter in real time with `/`.
- **File operations**: Multi-select, copy/cut/paste, move to Trash or permanently delete, create, and inline rename.
- **Side-by-side previews**: Asynchronous syntax highlighting, inline images (Kitty, iTerm2, Sixel, half-blocks), video thumbnails, and directory listings.
- **Editor & shell integration**: Open files in `$VISUAL`/`$EDITOR` and optionally `cd` on exit via `--cwd-file`.
- **Customizable**: 7 built-in themes plus optional `config.toml`, `themes/*.toml`, and `keybinds.toml`.

## Installation

**Linux & macOS (Shell):**

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/aissasell/lsi/releases/latest/download/lsi-installer.sh | sh
```

**Windows (PowerShell):**

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/aissasell/lsi/releases/latest/download/lsi-installer.ps1 | iex"
```

**From source (Cargo):**

```sh
cargo install --git https://github.com/aissasell/lsi.git
```

A [Nerd Font](https://www.nerdfonts.com/) is recommended for icons (or pass `--no-icons`). See [Getting Started](https://aissasell.github.io/lsi/guides/getting-started/) for prerequisites, minimal builds, and optional video preview tools.

## Quick Start

```sh
lsi                    # explore the current directory inline
lsi -la                # show hidden files and full metadata
lsi -l --preview -n 25 # open with the preview pane, max 25 rows
```

Press `?` inside `lsi` to view active keybindings, or run `lsi --help` for CLI flags.

## Documentation

Full guides and references are available at **[aissasell.github.io/lsi](https://aissasell.github.io/lsi/)**:

- [Getting Started](https://aissasell.github.io/lsi/guides/getting-started/) — installation, prerequisites, and first launch
- [Navigation & Usage](https://aissasell.github.io/lsi/guides/usage/) — tree navigation, file manipulation, search, and previews
- [Shell Integration](https://aissasell.github.io/lsi/guides/shell-integration/) — Fish, Bash, and Zsh wrappers to `cd` on exit
- [Configuration (`config.toml`)](https://aissasell.github.io/lsi/reference/configuration/) — defaults and preview settings
- [Keybindings (`keybinds.toml`)](https://aissasell.github.io/lsi/reference/keybindings/) — default shortcuts and custom remapping
- [Themes](https://aissasell.github.io/lsi/guides/themes/) & [Theme Reference (`themes/*.toml`)](https://aissasell.github.io/lsi/reference/theme-toml/) — built-in themes and custom palettes
- [CLI Options](https://aissasell.github.io/lsi/reference/cli/) — complete flag reference
- [Changelog](https://aissasell.github.io/lsi/changelog/) — release history

## License

Licensed under the [MIT License](LICENSE).
