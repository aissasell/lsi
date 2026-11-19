---
title: Theme Reference (theme.toml)
description: Complete list of all 32 color keys, accepted formats, and defaults for $HOME/.config/lsi/theme.toml.
---

When `$HOME/.config/lsi/theme.toml` exists, `lsi` registers it as the `custom` theme and selects it by default (while preserving all 7 built-in themes in the `T` / `c` cycle). Any omitted key inherits its value from `eza-dark`.

## Accepted Color Formats

- **Hex**: `"#10141a"` or `"10141a"`
- **RGB array**: `[16, 20, 26]`
- **`rgb()` string**: `"rgb(16, 20, 26)"`
- **Named colors**: `"black"`, `"red"`, `"green"`, `"yellow"`, `"blue"`, `"magenta"`, `"purple"`, `"cyan"`, `"gray"`, `"grey"`, `"darkgray"`, `"darkgrey"`, `"lightred"`, `"lightgreen"`, `"lightyellow"`, `"lightblue"`, `"lightmagenta"`, `"lightpurple"`, `"lightcyan"`, `"white"`, `"reset"`, `"default"`

## Color Keys

### UI Chrome

| Key | Default (`eza-dark`) | Description |
| --- | --- | --- |
| `bg` | `"#10141a"` | Main background color of the table and preview panes |
| `fg` | `"#dce0e8"` | Default foreground text color |
| `footer_bg` | `"#161b24"` | Background color of the footer status and hint bar |
| `footer_fg` | `"#b4becd"` | Foreground text color of the footer status bar |
| `border` | `"#3e4a5c"` | Border color for the table, preview separator, and modals |
| `selected_bg` | `"#2a384e"` | Background highlight color for the currently selected row |
| `selected_fg` | `"#ffffff"` | Foreground text color for the currently selected row |
| `accent` | `"#61afef"` | Accent color for selection index, modal borders, and search prompt |
| `muted` | `"#737d91"` | Muted color for column headers, hints, line numbers, and empty states |

### Entry Types

| Key | Default (`eza-dark`) | Description |
| --- | --- | --- |
| `dir_fg` | `"#61afef"` | Directory names and directory icons |
| `symlink_fg` | `"#56b6c2"` | Symbolic link names, icons, and valid link targets |
| `broken_symlink_fg` | `"#e06c75"` | Broken symbolic links and error notices |
| `exec_fg` | `"#98c379"` | Executable files and executable icons |
| `hidden_fg` | `"#808a9c"` | Hidden files (dotfiles) |
| `file_fg` | `"#dce0e8"` | Regular files without a specialized extension category |

### Extension Categories

| Key | Default (`eza-dark`) | Description |
| --- | --- | --- |
| `code_fg` | `"#e5c07b"` | Source code files and status messages |
| `config_fg` | `"#c678dd"` | Configuration and data files (`json`, `toml`, `yaml`, `sql`, etc.) |
| `doc_fg` | `"#61afef"` | Document files (`md`, `txt`, `pdf`, etc.) |
| `archive_fg` | `"#e06c75"` | Archive and package files (`zip`, `tar`, `gz`, etc.) |
| `media_fg` | `"#c678dd"` | Image, audio, video, and font files |

### Permissions Column

| Key | Default (`eza-dark`) | Description |
| --- | --- | --- |
| `perm_dir` | `"#61afef"` | Entry type characters (`d`, `l`, `s`, `p`, `b`, `c`) |
| `perm_read` | `"#e5c07b"` | Read permission bit (`r`) |
| `perm_write` | `"#e06c75"` | Write permission bit (`w`) |
| `perm_exec` | `"#98c379"` | Execute permission bit (`x`) |
| `perm_special` | `"#c678dd"` | Special permission bits (`s`, `S`, `t`, `T`) |
| `perm_none` | `"#5c6370"` | Unset permission bit (`-`) |

### Size, Ownership & Timestamp Columns

| Key | Default (`eza-dark`) | Description |
| --- | --- | --- |
| `size_number` | `"#98c379"` | Numeric part of file sizes |
| `size_unit` | `"#699b5f"` | Unit suffix of file sizes (`B`, `K`, `M`, `G`, `T`) |
| `owner_self` | `"#e5c07b"` | Owner username when owned by the current user |
| `owner_other` | `"#d19a66"` | Owner username when owned by another user |
| `group_fg` | `"#abb2bf"` | Group name in the ownership column |
| `date_fg` | `"#56b6c2"` | Modification timestamps (`YYYY-MM-DD HH:MM`) |

## Full Example

```toml
# $HOME/.config/lsi/theme.toml
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
