---
title: Keybindings
description: Complete keyboard and mouse shortcut reference and $HOME/.config/lsi/keybinds.toml customization guide for lsi.
---

All keyboard shortcuts in `lsi` can be customized in `$HOME/.config/lsi/keybinds.toml`. Both the footer keybind hints and the `?` help modal update automatically to reflect your active bindings.

## Customizing (`keybinds.toml`)

Create `$HOME/.config/lsi/keybinds.toml` and override only the actions you want to change—omitted actions keep their defaults.

- **Single key or multiple keys**: Assign either a single string (`rename_item = "F2"`) or an array (`move_down = ["j", "Down"]`).
- **Unbind an action**: Assign an empty array (`toggle_hidden = []`).
- **Conflict resolution**: Overriding an action automatically strips the newly assigned key(s) from any untouched default bindings in the same mode (while leaving sibling modes intact), so partial remaps work without redefining the rest of the file.
- **Invalid config safety**: Unknown field names or invalid key specs report a warning in the footer and fall back to defaults.

### Supported Key Names & Modifiers

- **Modifiers**: Combine `Ctrl+` (`Control+`), `Alt+` (`Meta+`), and `Shift+` before a key name (for example `"Ctrl+d"`, `"Shift+Delete"`, `"Ctrl+Shift+r"`, `"Ctrl++"`).
- **Printable characters**: Any single character such as `"j"`, `"P"`, `"."`, `"?"`, `"~"`, `"/"`, `"+"`, `"-"`, or `" "`.
- **Named keys**: `Enter` (`Return`, `CR`), `Esc` (`Escape`), `Space` (`Spc`), `Tab`, `BackTab` (or `Shift+Tab`), `Backspace` (`BS`), `Delete` (`Del`), `Insert` (`Ins`), `Up`, `Down`, `Left`, `Right`, `Home`, `End`, `PageUp` (`PgUp`), `PageDown` (`PgDn`, `PgDown`), `Plus`, `Minus`, and `F1`–`F24`.

## Navigation & History

| `keybinds.toml` | Key / Input | Action |
| --- | --- | --- |
| `move_down` | `j` / `Down` | Move selection down 1 row |
| `move_up` | `k` / `Up` | Move selection up 1 row |
| `—` | Mouse Scroll Up / Down | Scroll selection up / down |
| `page_down` | `Ctrl+d` / `PageDown` | Move selection down 10 rows |
| `page_up` | `Ctrl+u` / `PageUp` | Move selection up 10 rows |
| `jump_top` | `g` / `Home` | Jump to first entry |
| `jump_bottom` | `G` / `End` | Jump to last entry |
| `go_parent` | `h` / `Left` / `Backspace` | Go to parent directory (selects the folder you exited) |
| `history_back` | `-` / `[` / `b` | Navigate back in directory history |
| `history_forward` | `+` / `=` / `]` / `w` | Navigate forward in directory history |
| `jump_home` | `~` | Jump to `$HOME` |
| `refresh` | `F5` / `Ctrl+r` | Refresh current directory |

## Opening Files & Folders

| `keybinds.toml` | Key | Action |
| --- | --- | --- |
| `open` | `l` / `Right` / `Enter` | Enter directory, open text file in `$VISUAL` / `$EDITOR` inside the current terminal, or open binary/media file with system opener |
| `open_editor` | `e` | Unconditionally open selected file in `$VISUAL` / `$EDITOR` (fallback `vi` / `nano`) |
| `open_system` | `O` | Open selected file with system desktop opener (`xdg-open`) |

## File Manipulation & Multi-Selection

| `keybinds.toml` | Key | Mode | Action |
| --- | --- | --- | --- |
| `toggle_select` | `Space` | Normal | Toggle multi-selection on current item and move down 1 row |
| `yank` | `y` | Normal | Yank (copy) selected items (or current item) |
| `cut` | `x` | Normal | Cut selected items (or current item) |
| `paste` | `p` | Normal | Paste yanked/cut files and directories into current directory |
| `trash` | `Delete` | Normal | Move selected items (or current item) to the Trash bin |
| `permanent_delete` | `Shift+Delete` | Normal | Prompt for permanent deletion of selected items (or current item) |
| `confirm_delete` | `y` / `Y` / `Enter` | Confirm Delete | Confirm permanent deletion |
| `cancel_delete` | `n` / `N` / `Esc` / `q` | Confirm Delete | Cancel permanent deletion |
| `create_item` | `a` | Normal | Create a new file or folder (end with `/` for folder) |
| `create_confirm` | `Enter` | Create Item | Create the file or folder |
| `create_cancel` | `Esc` | Create Item | Cancel new item creation |
| `rename_item` | `r` | Normal | Rename the highlighted file or folder (prefilled with full name & extension) |
| `—` | `Left` / `Right` / `Home` / `End` / `Backspace` / `Delete` | Rename Item | Built-in cursor movement and character deletion |
| `rename_clear` | `Ctrl+u` | Rename Item | Clear text before cursor |
| `rename_confirm` | `Enter` | Rename Item | Apply the new name |
| `rename_cancel` | `Esc` | Rename Item | Cancel renaming |

## View & Chrome Toggles

| `keybinds.toml` | Key | Action |
| --- | --- | --- |
| `toggle_hidden` | `.` | Toggle hidden files and folders |
| `toggle_permissions` | `P` | Toggle permissions column |
| `toggle_ownership` | `o` / `u` | Toggle owner and group columns |
| `toggle_time` | `t` | Toggle modification date and time column |
| `toggle_size` | `S` | Toggle size column |
| `toggle_icons` | `i` | Toggle Nerd Font icons |
| `toggle_long` | `L` | Toggle all long metadata columns (`P`, `o`, `t`, `S`) at once |
| `toggle_borders` | `B` | Toggle borders around table and preview panes |
| `toggle_footer` | `F` | Toggle footer status and keybind hint bar |
| `toggle_preview` | `v` / `Tab` | Toggle side-by-side preview pane |

## Sorting & Themes

| `keybinds.toml` | Key | Action |
| --- | --- | --- |
| `cycle_theme` | `T` / `c` | Cycle color themes (includes `custom` when `theme.toml` exists) |
| `cycle_sort` | `s` | Cycle sort field (`name` → `size` → `modified` → `ext` → `type`) |
| `toggle_reverse_sort` | `R` | Toggle reverse sort order |
| `toggle_dirs_first` | `d` | Toggle grouping directories before files |

## Search, Help & Exiting

| `keybinds.toml` | Key | Mode | Action |
| --- | --- | --- | --- |
| `search` | `/` | Normal | Activate live substring search/filter bar |
| `search_confirm` | `Enter` | Search | Lock current filter and return to normal navigation |
| `—` | `Backspace` | Search | Delete last character of filter query |
| `search_cancel` | `Esc` | Search | Clear search filter and return to normal navigation |
| `cancel` | `Esc` | Normal | Cancel multi-selection (if active), clear active filter (if non-empty), or exit without changing directory |
| `toggle_help` | `?` | Normal / Help | Toggle help modal |
| `close_help` | `Esc` / `Enter` | Help | Close help modal |
| `quit_cd` | `q` | Normal / Help | Exit `lsi` and write `current_dir` to `--cwd-file` (for shell `cd`) |
| `quit` | `Q` / `Ctrl+c` | Normal / Help (modified/special keys in all modes) | Exit `lsi` without changing directory |

## Full Example (`keybinds.toml`)

```toml
# $HOME/.config/lsi/keybinds.toml

# Navigation & History
move_down = ["j", "Down"]
move_up = ["k", "Up"]
page_down = ["Ctrl+d", "PageDown"]
page_up = ["Ctrl+u", "PageUp"]
jump_top = ["g", "Home"]
jump_bottom = ["G", "End"]
go_parent = ["h", "Left", "Backspace"]
history_back = ["-", "[", "b"]
history_forward = ["+", "=", "]", "w"]
jump_home = "~"
refresh = ["F5", "Ctrl+r"]

# Opening Files & Folders
open = ["l", "Right", "Enter"]
open_editor = "e"
open_system = "O"

# File Manipulation & Multi-Selection
toggle_select = "Space"
yank = "y"
cut = "x"
paste = "p"
trash = "Delete"
permanent_delete = "Shift+Delete"
confirm_delete = ["y", "Y", "Enter"]
cancel_delete = ["n", "N", "Esc", "q"]
create_item = "a"
create_confirm = "Enter"
create_cancel = "Esc"
rename_item = "r"
rename_clear = "Ctrl+u"
rename_confirm = "Enter"
rename_cancel = "Esc"

# View & Chrome Toggles
toggle_hidden = "."
toggle_permissions = "P"
toggle_ownership = ["o", "u"]
toggle_time = "t"
toggle_size = "S"
toggle_icons = "i"
toggle_long = "L"
toggle_borders = "B"
toggle_footer = "F"
toggle_preview = ["v", "Tab"]

# Sorting & Themes
cycle_theme = ["T", "c"]
cycle_sort = "s"
toggle_reverse_sort = "R"
toggle_dirs_first = "d"

# Search, Help & Exiting
search = "/"
search_confirm = "Enter"
search_cancel = "Esc"
cancel = "Esc"
toggle_help = "?"
close_help = ["Esc", "Enter"]
quit_cd = "q"
quit = ["Q", "Ctrl+c"]
```
