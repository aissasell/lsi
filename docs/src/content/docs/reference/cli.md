---
title: CLI Options
description: Complete command-line reference for lsi flags, aliases, value enums, and option precedence.
---

## Synopsis

```text
lsi [OPTIONS] [PATH]
```

- `[PATH]` — Starting directory or file path (defaults to the current working directory `.`). When pointing to a file, `lsi` opens its parent directory with that file pre-selected.

## Configuration Precedence

Options resolve in the following priority order (highest to lowest):

1. **Explicit CLI flags** (for example `--no-borders`, `--theme nord`, `-n 30`)
2. **`$HOME/.config/lsi/config.toml`** (custom themes resolved from `$HOME/.config/lsi/themes/<name>.toml`)
3. **Hard-coded defaults**

## Options Reference

| Flag | Alias / Values | Default | Description |
| --- | --- | --- | --- |
| `-a, --all` | `--hidden` | `false` | Show hidden files and folders (dotfiles) |
| `-l, --long` | | `false` | Enable permissions, ownership, time, and size columns |
| `-p, --permissions` | | `false` | Show Unix permissions column (`drwxr-xr-x`) |
| `-o, --ownership` | `--user` | `false` | Show owner and group columns |
| `-t, --time` | `--date` | `false` | Show modification date and time column |
| `-s, --size` | | `true` | Show file size column (overrides `--no-size`) |
| `--no-size` | | `false` | Hide the file size column initially |
| `--icons` | | `true` | Show Nerd Font icons (overrides `--no-icons` and `config.toml`) |
| `--no-icons` | | `false` | Disable Nerd Font icons |
| `--borders` | | `true` | Show borders around table and preview panes |
| `--no-borders` | | `false` | Hide borders around table and preview panes |
| `--footer` | | `true` | Show footer status bar and keybind hint bar |
| `--no-footer` | | `false` | Hide footer status bar and keybind hint bar |
| `--theme <THEME>` | `eza-dark`, `catppuccin`, `dracula`, `nord`, `gruvbox`, `tokyo-night`, `solarized-light`, or `<name>` in `themes/<name>.toml` | `eza-dark` | Initial color theme (falls back to `eza-dark` if custom file is missing) |
| `--sort <SORT>` | `name`, `size`, `modified`, `extension`, `type` | `name` | Initial sort field |
| `-r, --reverse` | | `false` | Reverse sort order |
| `--dirs-first` | | `true` | Group directories before files |
| `--no-dirs-first` | | `false` | Do not group directories before files |
| `--preview` | | `false` | Start with the side-by-side preview pane open |
| `-n, --max-rows <MAX_ROWS>` | `--height` | `20` | Maximum number of inline terminal rows to use (clamped to `>= 3`) |
| `--cwd-file <PATH>` | | none | Write the final working directory to `<PATH>` when exiting with `q` |
| `-h, --help` | | | Print help |
| `-V, --version` | | | Print version |
