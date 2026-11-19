---
title: Themes & Custom Palette
description: Use the 7 built-in color themes or define your own default custom theme in $HOME/.config/lsi/theme.toml.
---

`lsi` ships with 7 built-in themes and supports defining your own `custom` theme via `$HOME/.config/lsi/theme.toml`.

## Built-in Themes

| Theme Name | Style |
| --- | --- |
| `eza-dark` | Default dark palette modeled after `eza` colors |
| `catppuccin` | Catppuccin Mocha palette |
| `dracula` | Dracula dark palette |
| `nord` | Arctic bluish Nord palette |
| `gruvbox` | Warm retro Gruvbox dark palette |
| `tokyo-night` | Deep indigo Tokyo Night palette |
| `solarized-light` | Light Solarized palette |

### Selecting & Cycling Themes

- **CLI flag**: `lsi --theme catppuccin`
- **Config file**: Set `theme = "nord"` in `$HOME/.config/lsi/config.toml`
- **Runtime keybind**: Press `T` or `c` anytime while `lsi` is running to cycle through available themes.

## Defining a Custom Theme (`theme.toml`)

Create `$HOME/.config/lsi/theme.toml` to define your own palette.

When `$HOME/.config/lsi/theme.toml` exists and parses cleanly:

1. It is registered as the `custom` theme (`--theme custom`).
2. `custom` **automatically becomes the default theme** (unless overridden by `theme` in `config.toml` or `--theme` on the CLI).
3. All 7 built-in themes remain intact—pressing `T` or `c` cycles through `custom` and every built-in theme.

Any omitted color keys in `theme.toml` automatically fall back to `eza-dark`.

## Supported Color Formats

Every key in `theme.toml` accepts any of the following formats:

- **Hex strings**: `"#61afef"` or `"61afef"`
- **RGB arrays**: `[97, 175, 239]`
- **`rgb()` strings**: `"rgb(97, 175, 239)"`
- **Named terminal colors** (case-insensitive, hyphens/underscores ignored):
  - `"black"`, `"red"`, `"green"`, `"yellow"`, `"blue"`, `"magenta"` / `"purple"`, `"cyan"`, `"gray"` / `"grey"`
  - `"darkgray"` / `"darkgrey"`, `"lightred"`, `"lightgreen"`, `"lightyellow"`, `"lightblue"`, `"lightmagenta"` / `"lightpurple"`, `"lightcyan"`, `"white"`
  - `"reset"` / `"default"`

## Minimal Example

```toml
# $HOME/.config/lsi/theme.toml
bg = "reset"
fg = "#cdd6f4"
border = "#585b70"
selected_bg = "#45475a"
accent = "#89b4fa"
dir_fg = "#89b4fa"
exec_fg = "#a6e3a1"
```

See [Theme Reference (`theme.toml`)](/reference/theme-toml/) for the complete list of all 32 color keys.
