---
title: Themes & Custom Palette
description: Use the 7 built-in color themes or load named custom themes from $HOME/.config/lsi/themes/<name>.toml.
---

`lsi` ships with 7 built-in themes and supports custom themes stored as `.toml` files in `$HOME/.config/lsi/themes/`.

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

- **CLI flag**: `lsi --theme catppuccin` (or `--theme catppuccin-mocha-mauve`)
- **Config file**: Set `theme = "nord"` or `theme = "catppuccin-mocha-mauve"` in `$HOME/.config/lsi/config.toml`
- **Runtime keybind**: Press `T` or `c` anytime while `lsi` is running to cycle through built-in themes (and the active custom theme when loaded).

## Defining a Custom Theme (`$HOME/.config/lsi/themes/<name>.toml`)

Place named `.toml` theme files inside `$HOME/.config/lsi/themes/` (for example `$HOME/.config/lsi/themes/my-theme.toml` or `$HOME/.config/lsi/themes/catppuccin-mocha-mauve.toml`) and select them in `config.toml` (`theme = "my-theme"`) or on the CLI (`lsi --theme my-theme`):

1. **Resolution & fallback**: When `theme = "my-theme"` is configured, `lsi` looks for `$HOME/.config/lsi/themes/my-theme.toml`. If the file does not exist or fails to parse, `lsi` falls back to `eza-dark`.
2. **Partial overrides**: Any omitted color keys automatically fall back to their defaults in `eza-dark`.
3. **Built-in names always take priority**: All 7 built-in theme names are protected—any file in `$HOME/.config/lsi/themes/` with a built-in name is ignored. For example, `$HOME/.config/lsi/themes/catppuccin.toml` is ignored and `catppuccin` continues to use the built-in theme, whereas `$HOME/.config/lsi/themes/catppuccin-mocha-mauve.toml` is loaded as custom when `theme = "catppuccin-mocha-mauve"`.
4. **Cycling**: When a custom theme is loaded, pressing `T` or `c` cycles through all 7 built-in themes and your loaded custom theme.

## Supported Color Formats

Every key in a custom theme file accepts any of the following formats:

- **Hex strings**: `"#61afef"` or `"61afef"`
- **RGB arrays**: `[97, 175, 239]`
- **`rgb()` strings**: `"rgb(97, 175, 239)"`
- **Named terminal colors** (case-insensitive, hyphens/underscores ignored):
  - `"black"`, `"red"`, `"green"`, `"yellow"`, `"blue"`, `"magenta"` / `"purple"`, `"cyan"`, `"gray"` / `"grey"`
  - `"darkgray"` / `"darkgrey"`, `"lightred"`, `"lightgreen"`, `"lightyellow"`, `"lightblue"`, `"lightmagenta"` / `"lightpurple"`, `"lightcyan"`, `"white"`
  - `"reset"` / `"default"`

## Minimal Example

```toml
# $HOME/.config/lsi/themes/my-theme.toml
bg = "reset"
fg = "#cdd6f4"
border = "#585b70"
selected_bg = "#45475a"
accent = "#cba6f7"
dir_fg = "#89b4fa"
exec_fg = "#a6e3a1"
```

See [Theme Reference (`themes/*.toml`)](/lsi/reference/theme-toml/) for the complete list of all 32 color keys.
