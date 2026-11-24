---
title: Navigation & Usage
description: Scroll through entries, navigate folder trees and history, open files in $EDITOR, toggle columns, preview files, and filter live.
---

`lsi` combines the visual clarity of `eza` with keyboard- and mouse-driven navigation right under your prompt.

## Inline Viewport & Dynamic Height

By default, `lsi` dynamically sizes its inline viewport to fit the number of visible entries plus UI chrome, up to `--max-rows` (`-n`, default `20`):

- **Standard mode** (`max_rows >= 6`): renders rounded borders (when `show_borders` is enabled), the column header row, visible entries, and the 2-line footer (when `show_footer` is enabled).
- **Compact mode** (`max_rows < 6`): omits outer borders and collapses the footer to a single status/hint line so small inline heights remain usable.

You can adjust the cap at launch (`lsi -n 12`), in `config.toml` (`max_rows = 12`), or toggle borders (`B`) and footer (`F`) interactively at runtime.

## Scrolling & Tree Navigation

- **Vertical scrolling**: Use `j` / `Down`, `k` / `Up`, mouse scroll wheel, `Ctrl+d` / `PgDn`, `Ctrl+u` / `PgUp`, `g` / `Home`, and `G` / `End`.
- **Entering folders**: Press `l`, `Right`, or `Enter` on a directory (or symlink pointing to a directory) to descend into it. `lsi` remembers the last selected entry per visited directory.
- **Parent folder**: Press `h`, `Left`, or `Backspace` to move up to the parent directory—automatically selecting the folder you exited.
- **Navigation history**: Move backward and forward through your visited directories with `-` / `[` / `b` (back) and `+` / `=` / `]` / `w` (forward).
- **Quick jumps & refresh**: Press `~` to jump to `$HOME`, or `F5` / `Ctrl+r` to reload the current directory.

## Opening Files (`$EDITOR` vs. System Opener)

- **Text files (`Enter` / `l` / `Right`)**: `lsi` checks if the selected file is a text file (no NUL bytes in the first 1024 bytes). If so, it temporarily clears the inline UI, restores normal terminal mode, and launches `$VISUAL` or `$EDITOR` (falling back to `vi` or `nano`) directly inside the **current terminal**. When you exit your editor, `lsi` refreshes the directory and resumes inline mode.
- **Force editor (`e`)**: Opens any selected file unconditionally in `$VISUAL` / `$EDITOR`.
- **System opener (`O` or binary/media files)**: Launches the desktop opener (`xdg-open` via `open::that_detached`) for binary/media files, or whenever you press `O`. If no GUI display (`DISPLAY` / `WAYLAND_DISPLAY`) is present, `lsi` falls back to opening text files in `$EDITOR` or opening the preview pane for binary files.

## Toggling Metadata Columns & UI Chrome

All launch flags can also be toggled live while browsing:

| Key | Toggles |
| --- | --- |
| `.` | Hidden files and folders (dotfiles) |
| `P` | Unix permissions (`drwxr-xr-x`) |
| `o` or `u` | Owner and group columns |
| `t` | Modification date and time (`YYYY-MM-DD HH:MM`) |
| `S` | File size column |
| `i` | Nerd Font icons |
| `L` | All long metadata columns (`P`, `o`, `t`, `S`) at once |
| `B` | Borders around table and preview panes |
| `F` | Footer status bar and keybind hints |

## File Manipulation & Multi-Selection

- **Multi-select (`Space`)**: Press `Space` to mark or unmark the highlighted file or folder and advance down one row (`●` marks selected items). Press `Esc` to clear the selection.
- **Copy (`y`), Cut (`x`), and Paste (`p`)**: Yank or cut the multi-selected items (or the currently highlighted item if none are marked), navigate to a target folder, and press `p` to paste. Name collisions automatically receive `_copy` / `_copy_2` suffixes.
- **Trash (`Delete`)**: Moves selected items to your FreeDesktop.org Trash bin (`$XDG_DATA_HOME/Trash` or `$HOME/.local/share/Trash`) with `.trashinfo` metadata.
- **Permanent Delete (`Shift+Delete`)**: Prompts with `(y/N)` in the footer before permanently deleting files or directory trees (symlinks are unlinked without deleting their target).
- **New File or Folder (`a`)**: Opens a `NEW` prompt in the footer — type a name and press `Enter` to create a file, or end the name with `/` (e.g., `newdir/`) to create a folder (nested relative paths create missing parent folders automatically). Press `Esc` to cancel.
- **Rename (`r`)**: Opens a `RENAME` prompt in the footer prefilled with the highlighted file or folder's full name (including extension), allowing edits to any part of the name or extension using `Left` / `Right` / `Home` / `End`, `Backspace` / `Delete`, or `Ctrl+u`. Press `Enter` to apply or `Esc` to cancel.

## Side-by-Side Preview & Live Search

- **Preview pane (`v` or `Tab`)**: Splits the inline area (`60%` table / `40%` preview when width `>= 60`) to asynchronously display an icon-annotated directory listing, up to 200 lines / 64 KiB of syntax-highlighted text (`syntect`, guarded against minified files), inline image previews (Kitty Graphics, iTerm2, Sixel, or Unicode half-blocks via `ratatui-image`), or video thumbnails (looking up cached thumbnails under `$XDG_CACHE_HOME` or `$HOME/.cache/thumbnails` when `$XDG_CACHE_HOME` is unset, extracting via `ffmpegthumbnailer` or `ffmpeg` on cache miss, and falling back to structured `ffprobe` / file metadata). Use `J` / `Alt+Down` and `K` / `Alt+Up` to scroll long text, directory, or metadata previews.
- **Live filter (`/`)**: Enters search mode and filters entries in real time by case-insensitive substring match:
  - Press `Enter` to lock the filter and resume navigating the filtered results.
  - Press `Esc` to clear the filter and return to normal navigation.

## Sorting

- `s`: Cycle sort field (`name` → `size` → `modified` → `ext` → `type`).
- `R`: Reverse sort direction.
- `d`: Toggle grouping directories before files.
