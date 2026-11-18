# `lsi` — Interactive `eza`-style Terminal File Explorer

`lsi` is an interactive directory listing tool written in Rust with `ratatui` and `crossterm`. Inspired by `eza`, it renders directory contents inline directly below your shell prompt while letting you scroll, preview files, navigate back and forth through the folder tree and history, toggle metadata columns, switch color themes, and open files right in your terminal editor.

## Features

- **Inline Viewport with Dynamic Height**: Renders cleanly inline below your prompt instead of taking over the alternate screen, growing dynamically with directory entries up to `--max-rows` (`-n`, default `20`).
- **`eza`-Inspired Metadata Columns**:
  - Per-character colorized Unix permissions (`drwxr-xr-x`)
  - Human-readable file sizes (`B`, `K`, `M`, `G`, `T`)
  - Resolved owner & group names
  - Modification date & time (`YYYY-MM-DD HH:MM`)
- **Nerd Font Icons**: Distinct glyphs and semantic colors for special directories (`.git`, `.github`, `src`, `node_modules`, `target`, `docs`, etc.), exact filenames (`Cargo.toml`, `Dockerfile`, `Makefile`, `README.md`, `.env`, etc.), and file extensions.
- **Interactive Tree & History Navigation**: Step into folders, climb up to parent directories (keeping the exited folder selected), and move back/forward through your navigation history.
- **In-Terminal Editor Integration**: Pressing `Enter` or `l` on a text file suspends the inline UI and opens it directly in `$VISUAL` / `$EDITOR` (e.g., `nvim`) inside the current terminal, returning smoothly when you exit. Binary/media files open via the system opener (or press `O` explicitly).
- **Side-by-Side Preview Pane**: Toggle a live preview pane (`v` or `Tab`) for text files and directory contents.
- **Live Search & Sorting**: Filter entries in real time with `/`, and sort by name, size, modified time, extension, or type.
- **Built-in Color Themes**: Switch between 7 color themes on the fly.

## Prerequisites

- **Rust toolchain** (1.88+ / Cargo)
- **A Nerd Font** configured in your terminal emulator for icon glyphs (or pass `--no-icons` / toggle `i` at runtime)

## Installation

Build and install using cargo:

```bash
cargo install --git https://github.com/aissasell/lsi.git
```

## Usage

```bash
# Explore the current directory inline
lsi

# Show hidden files and full long metadata (-p, -o, -t, -s)
lsi -la

# Start with the preview pane open and cap inline height to 25 rows
lsi -l --preview -n 25

# Launch in a specific directory with a chosen theme and sort order
lsi --theme catppuccin --sort modified --reverse ~/Projects
```

### CLI Options

| Flag | Alias / Values | Description |
| --- | --- | --- |
| `[PATH]` | default: `.` | Starting directory or file path |
| `-a, --all` | `--hidden` | Show hidden files and folders (dotfiles) |
| `-l, --long` | | Enable permissions, ownership, time, and size columns |
| `-p, --permissions` | | Show Unix permissions column |
| `-o, --ownership` | `--user` | Show owner and group columns |
| `-t, --time` | `--date` | Show modification date and time column |
| `-s, --size` | `--no-size` | Show (or hide) file size column (shown by default) |
| `--no-icons` | | Disable Nerd Font icons |
| `--theme <THEME>` | `eza-dark` (default), `catppuccin`, `dracula`, `nord`, `gruvbox`, `tokyo-night`, `solarized-light` | Initial color theme |
| `--sort <SORT>` | `name` (default), `size`, `modified`, `extension`, `type` | Initial sort field |
| `-r, --reverse` | | Reverse sort order |
| `--no-dirs-first` | | Do not group directories before files |
| `--preview` | | Start with the side-by-side preview pane open |
| `-n, --max-rows <N>` | `--height`, default: `20` | Maximum inline terminal rows to use |
| `-h, --help` | | Print help |
| `-V, --version` | | Print version |

## Keybindings

### Navigation & History

| Key | Action |
| --- | --- |
| `j` / `Down`, `k` / `Up` | Scroll down / up (mouse wheel also supported) |
| `Ctrl+d` / `PgDn`, `Ctrl+u` / `PgUp` | Page down / up |
| `g` / `Home`, `G` / `End` | Jump to first / last entry |
| `l` / `Right` / `Enter` | Enter directory or open text file in `$EDITOR` (binary/media via system opener) |
| `e` | Open selected file in `$VISUAL` / `$EDITOR` (fallback `vi` / `nano`) |
| `O` | Open selected file with system desktop opener (`xdg-open`) |
| `h` / `Left` / `Backspace` | Go to parent directory |
| `-` / `[` / `b` | Navigate back in history |
| `+` / `=` / `]` / `w` | Navigate forward in history |
| `~` | Jump to `$HOME` |
| `R` / `F5` | Refresh current directory |

### View, Sorting, Themes & Search

| Key | Action |
| --- | --- |
| `.` / `a` | Toggle hidden files and folders |
| `p` | Toggle permissions column |
| `o` / `u` | Toggle owner & group columns |
| `t` | Toggle modification date & time column |
| `S` | Toggle size column |
| `i` | Toggle Nerd Font icons |
| `L` | Toggle all long metadata columns at once |
| `T` / `c` | Cycle color themes |
| `s` | Cycle sort field (`name`, `size`, `modified`, `ext`, `type`) |
| `r` | Toggle reverse sort order |
| `d` | Toggle directories-first grouping |
| `v` / `Tab` | Toggle side-by-side preview pane |
| `/` | Activate live filter (`Enter` to lock filter, `Esc` to clear) |
| `Esc` | Clear active filter or status message |
| `?` | Toggle help overlay |
| `q` / `Ctrl+c` | Exit `lsi` |

## Themes

Cycle themes anytime with `T` or `c`, or pass `--theme <name>` on launch:

- `eza-dark` (default)
- `catppuccin`
- `dracula`
- `nord`
- `gruvbox`
- `tokyo-night`
- `solarized-light`
