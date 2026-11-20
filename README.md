<div align="center">
  <img src="assets/logo.svg" alt="lsi logo" width="96" />
  <h1><code>lsi</code> — Interactive <code>eza</code>-style Terminal File Explorer</h1>
</div>

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
| `--icons` / `--no-icons` | | Show or disable Nerd Font icons (enabled by default) |
| `--borders` / `--no-borders` | | Show or hide borders around table and preview panes (enabled by default) |
| `--footer` / `--no-footer` | | Show or hide the footer status and keybind hint bar (enabled by default) |
| `--theme <THEME>` | `custom` (when `theme.toml` exists), `eza-dark` (default), `catppuccin`, `dracula`, `nord`, `gruvbox`, `tokyo-night`, `solarized-light` | Initial color theme |
| `--sort <SORT>` | `name` (default), `size`, `modified`, `extension`, `type` | Initial sort field |
| `-r, --reverse` | | Reverse sort order |
| `--dirs-first` / `--no-dirs-first` | | Group (or do not group) directories before files |
| `--preview` | | Start with the side-by-side preview pane open |
| `-n, --max-rows <N>` | `--height`, default: `20` | Maximum inline terminal rows to use |
| `--cwd-file <PATH>` | | Write the final working directory to this file when exiting with `q` |
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
| `F5` / `Ctrl+r` | Refresh current directory |

### File Manipulation & Multi-Selection

| Key | Action |
| --- | --- |
| `Space` | Toggle multi-selection on the highlighted item and move down 1 row |
| `y` | Yank (copy) selected items (or current item) |
| `x` | Cut selected items (or current item) |
| `p` | Paste yanked/cut files and directories into the current directory |
| `Delete` | Move selected items (or current item) to the Trash bin |
| `a` | Create a new file or folder (end name with `/` for folder; `Enter` creates, `Esc` cancels) |
| `r` | Rename the highlighted file or folder (prefilled with full name & extension; `Enter` renames, `Esc` cancels) |
| `Shift+Delete` | Permanently delete selected items (or current item) after `(y/N)` confirmation |
| `Esc` | Cancel multi-selection (if active), clear filter (if active), or exit without `cd` |

### View, Sorting, Themes & Search

| Key | Action |
| --- | --- |
| `.` | Toggle hidden files and folders |
| `P` | Toggle permissions column |
| `o` / `u` | Toggle owner & group columns |
| `t` | Toggle modification date & time column |
| `S` | Toggle size column |
| `i` | Toggle Nerd Font icons |
| `B` | Toggle borders around table and preview panes |
| `F` | Toggle footer status and keybind hint bar |
| `L` | Toggle all long metadata columns at once |
| `T` / `c` | Cycle color themes (includes `custom` when `theme.toml` exists) |
| `s` | Cycle sort field (`name`, `size`, `modified`, `ext`, `type`) |
| `R` | Toggle reverse sort order |
| `d` | Toggle directories-first grouping |
| `v` / `Tab` | Toggle side-by-side preview pane |
| `/` | Activate live filter (`Enter` to lock filter, `Esc` to clear) |
| `Esc` | Cancel active selection, clear active filter, or exit without changing directory |
| `?` | Toggle help overlay |
| `q` | Exit `lsi` and change directory (writes `--cwd-file`) |
| `Q` / `Ctrl+c` | Exit `lsi` without changing directory |

## Shell Integration (Change Directory on Exit)

Use `--cwd-file` in a shell wrapper function so pressing `q` changes your shell's working directory to the folder you navigated to, while pressing `Q` or `Esc` exits without changing directories.

### Fish

```fish
function l
    set tmp (mktemp -t "lsi-cwd.XXXXXX")
    command lsi $argv --cwd-file="$tmp"
    if read -z cwd <"$tmp"; and [ "$cwd" != "$PWD" ]; and test -d "$cwd"
        builtin cd -- "$cwd"
    end
    rm -f -- "$tmp"
end
```

### Bash / Zsh

```bash
function l() {
    local tmp="$(mktemp -t "lsi-cwd.XXXXXX")" cwd
    command lsi "$@" --cwd-file="$tmp"
    IFS= read -r -d '' cwd < "$tmp"
    if [ -n "$cwd" ] && [ "$cwd" != "$PWD" ] && [ -d "$cwd" ]; then
        builtin cd -- "$cwd"
    fi
    rm -f -- "$tmp"
}
```

## Configuration (`config.toml` & `theme.toml`)

`lsi` looks for optional TOML configuration files in `$HOME/.config/lsi/`:

- `$HOME/.config/lsi/config.toml` — Overrides hard-coded defaults (explicit CLI flags take priority over `config.toml`).
- `$HOME/.config/lsi/theme.toml` — Defines a `custom` theme. When present, `custom` automatically becomes the default theme, while all built-in themes remain available when cycling with `T` / `c` or `--theme`.

### Example `config.toml`

```toml
show_borders = true
show_footer = true
show_hidden = false
long = false
show_permissions = false
show_ownership = false
show_time = false
show_size = true
show_icons = true
show_preview = false
dirs_first = true
reverse = false
sort = "name"        # name, size, modified, extension (or ext), type
# theme = "eza-dark" # optional override: custom, eza-dark, catppuccin, dracula, nord, gruvbox, tokyo-night, solarized-light
max_rows = 20
```

### Example `theme.toml`

Colors accept hex strings (`"#rrggbb"` / `"rrggbb"`), RGB arrays (`[r, g, b]`), `"rgb(r, g, b)"`, or terminal color names (`"blue"`, `"light-cyan"`, `"reset"`, etc.). Any omitted fields fall back to `eza-dark`:

```toml
bg = "#10141a"
fg = "#dce0e8"
footer_bg = "#161b24"
footer_fg = "#b4becd"
border = "#3e4a5c"
selected_bg = "#2a384e"
selected_fg = "#ffffff"
accent = "#61afef"
muted = "#737d91"

dir_fg = "#61afef"
symlink_fg = "#56b6c2"
broken_symlink_fg = "#e06c75"
exec_fg = "#98c379"
hidden_fg = "#808a9c"
file_fg = "#dce0e8"

code_fg = "#e5c07b"
config_fg = "#c678dd"
doc_fg = "#61afef"
archive_fg = "#e06c75"
media_fg = "#c678dd"

perm_dir = "#61afef"
perm_read = "#e5c07b"
perm_write = "#e06c75"
perm_exec = "#98c379"
perm_special = "#c678dd"
perm_none = "#5c6370"

size_number = "#98c379"
size_unit = "#699b5f"

owner_self = "#e5c07b"
owner_other = "#d19a66"
group_fg = "#abb2bf"
date_fg = "#56b6c2"
```

## Themes

Cycle themes anytime with `T` or `c`, or pass `--theme <name>` on launch:

- `custom` (loaded from `$HOME/.config/lsi/theme.toml` and used as default when present)
- `eza-dark` (default when no `theme.toml` exists)
- `catppuccin`
- `dracula`
- `nord`
- `gruvbox`
- `tokyo-night`
- `solarized-light`
