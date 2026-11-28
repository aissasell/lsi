#!/usr/bin/env python3
"""Update GitHub Release notes with Copr/APT install snippets and .deb/.rpm download rows."""

from __future__ import annotations

import argparse
import re
from pathlib import Path

SNIPPETS_START = "<!-- linux-install-snippets:start -->"
SNIPPETS_END = "<!-- linux-install-snippets:end -->"

INSTALL_SNIPPETS = f"""{SNIPPETS_START}
### Install on Fedora via Copr

```sh
sudo dnf copr enable aissasell/lsi
sudo dnf install lsi
```

### Install on Debian / Ubuntu via APT

```sh
sudo install -d -m 0755 /etc/apt/keyrings
curl -fsSL https://aissasell.github.io/apt/pubkey.gpg | sudo gpg --dearmor --yes -o /etc/apt/keyrings/aissasell.gpg
echo "deb [signed-by=/etc/apt/keyrings/aissasell.gpg] https://aissasell.github.io/apt stable main" | sudo tee /etc/apt/sources.list.d/aissasell.list
sudo apt update && sudo apt install lsi
```
{SNIPPETS_END}"""

PACKAGE_ORDER = (
    ("_amd64.deb", "x64 Debian / Ubuntu (.deb)"),
    ("_arm64.deb", "ARM64 Debian / Ubuntu (.deb)"),
    (".x86_64.rpm", "x64 Fedora / RHEL (.rpm)"),
    (".aarch64.rpm", "ARM64 Fedora / RHEL (.rpm)"),
)


def collect_package_rows(dist_dir: Path, repo: str, tag: str) -> list[str]:
    """Return sorted Markdown download table rows for .deb and .rpm artifacts."""
    files = sorted(
        p.name for p in dist_dir.iterdir() if p.is_file() and p.name.endswith((".deb", ".rpm"))
    )
    rows: list[str] = []
    seen: set[str] = set()
    base_url = f"https://github.com/{repo}/releases/download/{tag}"

    for suffix, platform in PACKAGE_ORDER:
        for name in files:
            if name.endswith(suffix) and name not in seen:
                seen.add(name)
                rows.append(
                    f"| [{name}]({base_url}/{name}) | {platform} | [checksum]({base_url}/{name}.sha256) |"
                )

    for name in files:
        if name not in seen:
            platform = "Debian / Ubuntu (.deb)" if name.endswith(".deb") else "Fedora / RHEL (.rpm)"
            rows.append(
                f"| [{name}]({base_url}/{name}) | {platform} | [checksum]({base_url}/{name}.sha256) |"
            )

    return rows


def upsert_install_snippets(body: str) -> str:
    """Insert or replace the Copr/APT install block right before `## Download `."""
    cleaned = re.sub(
        rf"\n*{re.escape(SNIPPETS_START)}.*?{re.escape(SNIPPETS_END)}\n*",
        "\n\n",
        body,
        flags=re.DOTALL,
    ).strip("\n")

    download_match = re.search(r"^## Download\b", cleaned, flags=re.MULTILINE)
    if download_match:
        before = cleaned[: download_match.start()].rstrip("\n")
        after = cleaned[download_match.start() :].lstrip("\n")
        return f"{before}\n\n{INSTALL_SNIPPETS}\n\n{after}"

    if cleaned:
        return f"{cleaned}\n\n{INSTALL_SNIPPETS}"
    return INSTALL_SNIPPETS


def upsert_download_rows(body: str, rows: list[str], tag: str) -> str:
    """Insert or replace .deb/.rpm rows inside the `## Download` Markdown table."""
    if not rows:
        return body

    lines = body.split("\n")
    download_idx = next(
        (i for i, line in enumerate(lines) if re.match(r"^## Download\b", line)),
        None,
    )

    if download_idx is None:
        version = tag.lstrip("v")
        table = "\n".join(
            [
                f"## Download lsi {version}",
                "",
                "|  File  | Platform | Checksum |",
                "|--------|----------|----------|",
                *rows,
            ]
        )
        return f"{body.rstrip()}\n\n{table}" if body.strip() else table

    section_end = next(
        (i for i in range(download_idx + 1, len(lines)) if re.match(r"^##\s+", lines[i])),
        len(lines),
    )

    pkg_row_re = re.compile(r"^\|\s*\[[^\]]+\.(?:deb|rpm)\]\(")
    section_lines = [
        line for line in lines[download_idx:section_end] if not pkg_row_re.match(line)
    ]

    last_table_idx = max(
        (i for i, line in enumerate(section_lines) if line.lstrip().startswith("|")),
        default=None,
    )
    if last_table_idx is not None:
        updated_section = (
            section_lines[: last_table_idx + 1] + rows + section_lines[last_table_idx + 1 :]
        )
    else:
        updated_section = section_lines + [
            "",
            "|  File  | Platform | Checksum |",
            "|--------|----------|----------|",
            *rows,
        ]

    return "\n".join(lines[:download_idx] + updated_section + lines[section_end:])


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--body-file", required=True, type=Path)
    parser.add_argument("--dist-dir", required=True, type=Path)
    parser.add_argument("--repo", required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    body = args.body_file.read_text(encoding="utf-8").replace("\r\n", "\n")
    rows = collect_package_rows(args.dist_dir, args.repo, args.tag)
    updated = upsert_install_snippets(body)
    updated = upsert_download_rows(updated, rows, args.tag).rstrip() + "\n"
    args.output.write_text(updated, encoding="utf-8")


if __name__ == "__main__":
    main()
