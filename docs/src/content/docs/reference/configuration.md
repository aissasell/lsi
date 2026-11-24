---
title: Configuration (config.toml)
description: Reference for all options and aliases supported in $HOME/.config/lsi/config.toml.
---

`lsi` reads optional startup defaults from `$HOME/.config/lsi/config.toml` (alongside [`themes/*.toml`](/lsi/reference/theme-toml/) for named custom colors and [`keybinds.toml`](/lsi/reference/keybindings/) for custom keyboard shortcuts). Values set in `config.toml` take priority over hard-coded defaults, while explicit CLI flags override `config.toml`.

If `config.toml` contains a syntax or type error, `lsi` falls back to defaults.

## Options

| Key | Aliases | Values / Type | Default | Description |
| --- | --- | --- | --- | --- |
| `show_borders` | `borders` | `true` \| `false` | `true` | Show or hide borders around the file table and preview pane |
| `show_footer` | `footer` | `true` \| `false` | `true` | Show or hide the footer status bar and keybind hint bar |
| `show_hidden` | `all`, `hidden` | `true` \| `false` | `false` | Show or hide dotfiles (hidden files and folders) |
| `long` | | `true` \| `false` | `false` | Enable permissions, ownership, time, and size columns together |
| `show_permissions` | `permissions` | `true` \| `false` | `false` | Show or hide the Unix permissions column |
| `show_ownership` | `ownership`, `user` | `true` \| `false` | `false` | Show or hide owner and group columns |
| `show_time` | `time`, `date` | `true` \| `false` | `false` | Show or hide the modification date and time column |
| `show_size` | `size` | `true` \| `false` | `true` | Show or hide the file size column |
| `show_icons` | `icons` | `true` \| `false` | `true` | Show or hide Nerd Font icons |
| `show_preview` | `preview` | `true` \| `false` | `false` | Open with the side-by-side preview pane enabled |
| `syntax_highlight` | | `true` \| `false` | `true` | Enable pure-Rust `syntect` syntax highlighting in text previews |
| `image_preview` | | `true` \| `false` | `true` | Decode & render raster images (`png`, `jpeg`, `gif`, `webp`, `bmp`, `ico`) in the preview pane |
| `image_protocol` | | `"auto"` \| `"kitty"` \| `"sixel"` \| `"iterm2"` \| `"halfblocks"` \| `"off"` (`"none"`) | `"auto"` | Preferred terminal graphics protocol for image previews |
| `preview_max_file_mb` | | integer (`>= 1`) | `20` | Maximum image file size (MiB) before skipping decode |
| `dirs_first` | | `true` \| `false` | `true` | Group directories before files when sorting |
| `reverse` | `sort_reverse` | `true` \| `false` | `false` | Reverse sort order |
| `sort` | `sort_field` | `"name"` \| `"size"` \| `"modified"` \| `"extension"` \| `"ext"` \| `"type"` | `"name"` | Field used to sort entries |
| `theme` | | `"eza-dark"` \| `"catppuccin"` \| `"dracula"` \| `"nord"` \| `"gruvbox"` \| `"tokyo-night"` \| `"solarized-light"` \| any `<name>` matching `$HOME/.config/lsi/themes/<name>.toml` | `"eza-dark"` | Initial color theme (falls back to `"eza-dark"` if missing; built-in names ignore files in `themes/`) |
| `max_rows` | `height` | integer (`>= 3`) | `20` | Maximum inline terminal rows used by the UI |

## Full Example

```toml
# $HOME/.config/lsi/config.toml
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
syntax_highlight = true
image_preview = true
image_protocol = "auto"
preview_max_file_mb = 20
dirs_first = true
reverse = false
sort = "name"
# theme = "eza-dark"
max_rows = 20
```
