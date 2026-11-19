---
title: Getting Started
description: Install lsi, configure a Nerd Font, and launch your first interactive inline directory listing.
---

`lsi` is an interactive `eza`-inspired terminal file explorer written in Rust. Unlike `ls` or `eza` (which print static text and exit) or fullscreen file managers (which take over your entire terminal window), `lsi` renders an interactive table **inline directly below your shell prompt**.

When you exit, your terminal scrollback remains intact—and with [Shell Integration](/lsi/guides/shell-integration/) you can change your shell's working directory to the folder you browsed to.

## Prerequisites

- **Rust toolchain** (Rust 1.88+ and Cargo) if building from source
- **A Nerd Font** configured in your terminal emulator to render file and folder glyphs (or pass `--no-icons` / toggle `i` at runtime)

## Installation

Install directly from Git using Cargo:

```bash
cargo install --git https://github.com/aissasell/lsi.git
```

### Build from a Local Clone

```bash
git clone https://github.com/aissasell/lsi.git
cd lsi
cargo build --release
```

The release binary is placed at `target/release/lsi`. You can also install it into `~/.cargo/bin`:

```bash
cargo install --path .
```

## First Launch

Run `lsi` in any directory to open an interactive inline listing:

```bash
# Explore the current directory inline
lsi

# Show hidden files and full long metadata (permissions, owner/group, modified time, size)
lsi -la

# Start with the side-by-side preview pane open and cap inline height to 25 rows
lsi -l --preview -n 25

# Open in a specific directory with a theme and sort order
lsi --theme catppuccin --sort modified --reverse ~/Projects
```

Passing a file path (for example `lsi src/main.rs`) opens `lsi` in that file's parent directory with the file pre-selected.

## Next Steps

- Learn tree navigation, history, previews, and toggles in [Navigation & Usage](/lsi/guides/usage/).
- Set up `cd` on exit in [Shell Integration](/lsi/guides/shell-integration/).
- Customize defaults and colors in [Configuration (`config.toml`)](/lsi/reference/configuration/) and [Themes & Custom Palette](/lsi/guides/themes/).
