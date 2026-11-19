---
title: Shell Integration
description: Change your shell's current working directory when exiting lsi with q, or exit without changing directory using Q or Esc.
---

Because child processes cannot directly modify their parent shell's working directory, `lsi` supports a `--cwd-file <PATH>` flag (similar to `yazi`). When wrapped in a small shell function, you can browse directories interactively and choose whether to `cd` upon exiting.

## Exit Behavior

| Key | Behavior |
| --- | --- |
| `q` | Exits `lsi` and writes `current_dir` to `--cwd-file` so the shell wrapper executes `cd`. |
| `Q` | Exits `lsi` **without** writing `--cwd-file` (keeps your original shell directory). |
| `Esc` | Clears an active search filter if one is applied; otherwise exits **without** writing `--cwd-file`. |
| `Ctrl+c` | Exits `lsi` **without** writing `--cwd-file`. |

:::tip
Using `q` to jump into the folder you browsed to and `Q` / `Esc` to cancel back to where you started lets one wrapper function act as both an interactive `ls` and an interactive `cd`.
:::

## Shell Wrapper Functions

### Fish

Add this function to `~/.config/fish/config.fish` (or `~/.config/fish/functions/l.fish`):

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

### Bash

Add this function to `~/.bashrc`:

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

### Zsh

Add this function to `~/.zshrc`:

```zsh
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

After reloading your shell config, run `l` (with any `lsi` flags like `l -la` or `l --preview`), navigate into a folder, and press `q` to `cd` there—or press `Q` / `Esc` to exit without moving.
