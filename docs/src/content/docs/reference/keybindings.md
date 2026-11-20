---
title: Keybindings
description: Complete keyboard and mouse shortcut reference for navigating, toggling columns, previewing, filtering, and exiting lsi.
---

## Navigation & History

| Key / Input | Action |
| --- | --- |
| `j` / `Down` | Move selection down 1 row |
| `k` / `Up` | Move selection up 1 row |
| Mouse Scroll Up / Down | Scroll selection up / down |
| `Ctrl+d` / `PageDown` | Move selection down 10 rows |
| `Ctrl+u` / `PageUp` | Move selection up 10 rows |
| `g` / `Home` | Jump to first entry |
| `G` / `End` | Jump to last entry |
| `h` / `Left` / `Backspace` | Go to parent directory (selects the folder you exited) |
| `-` / `[` / `b` | Navigate back in directory history |
| `+` / `=` / `]` / `w` | Navigate forward in directory history |
| `~` | Jump to `$HOME` |
| `F5` / `Ctrl+r` | Refresh current directory |

## Opening Files & Folders

| Key | Action |
| --- | --- |
| `l` / `Right` / `Enter` | Enter directory, open text file in `$VISUAL` / `$EDITOR` inside the current terminal, or open binary/media file with system opener |
| `e` | Unconditionally open selected file in `$VISUAL` / `$EDITOR` (fallback `vi` / `nano`) |
| `O` | Open selected file with system desktop opener (`xdg-open`) |

## File Manipulation & Multi-Selection

| Key | Mode | Action |
| --- | --- | --- |
| `Space` | Normal | Toggle multi-selection on current item and move down 1 row |
| `y` | Normal | Yank (copy) selected items (or current item) |
| `x` | Normal | Cut selected items (or current item) |
| `p` | Normal | Paste yanked/cut files and directories into current directory |
| `Delete` | Normal | Move selected items (or current item) to the Trash bin |
| `Shift+Delete` | Normal | Prompt for permanent deletion of selected items (or current item) |
| `y` / `Y` / `Enter` | Confirm Delete | Confirm permanent deletion |
| `n` / `N` / `Esc` / `q` | Confirm Delete | Cancel permanent deletion |
| `a` | Normal | Create a new file or folder (end with `/` for folder) |
| `Enter` | Create Item | Create the file or folder |
| `Esc` | Create Item | Cancel new item creation |
| `r` | Normal | Rename the highlighted file or folder (prefilled with full name & extension) |
| `Left` / `Right` / `Home` / `End` / `Ctrl+u` | Rename Item | Move cursor or clear text before cursor |
| `Enter` | Rename Item | Apply the new name |
| `Esc` | Rename Item | Cancel renaming |

## View & Chrome Toggles

| Key | Action |
| --- | --- |
| `.` | Toggle hidden files and folders |
| `P` | Toggle permissions column |
| `o` / `u` | Toggle owner and group columns |
| `t` | Toggle modification date and time column |
| `S` | Toggle size column |
| `i` | Toggle Nerd Font icons |
| `L` | Toggle all long metadata columns (`P`, `o`, `t`, `S`) at once |
| `B` | Toggle borders around table and preview panes |
| `F` | Toggle footer status and keybind hint bar |
| `v` / `Tab` | Toggle side-by-side preview pane |

## Sorting & Themes

| Key | Action |
| --- | --- |
| `T` / `c` | Cycle color themes (includes `custom` when `theme.toml` exists) |
| `s` | Cycle sort field (`name` → `size` → `modified` → `ext` → `type`) |
| `R` | Toggle reverse sort order |
| `d` | Toggle grouping directories before files |

## Search, Help & Exiting

| Key | Mode | Action |
| --- | --- | --- |
| `/` | Normal | Activate live substring search/filter bar |
| `Enter` | Search | Lock current filter and return to normal navigation |
| `Backspace` | Search | Delete last character of filter query |
| `Esc` | Search | Clear search filter and return to normal navigation |
| `Esc` | Normal | Cancel multi-selection (if active), clear active filter (if non-empty), or exit without changing directory |
| `?` | Normal / Help | Toggle help modal (`Esc` / `Enter` also closes help modal) |
| `q` | Normal / Help | Exit `lsi` and write `current_dir` to `--cwd-file` (for shell `cd`) |
| `Q` / `Ctrl+c` | Normal / Help (`Ctrl+c` in all modes) | Exit `lsi` without changing directory |
