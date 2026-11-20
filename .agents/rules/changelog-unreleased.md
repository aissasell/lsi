---
trigger: always_on
---

# Changelog Rule for the `dev` Branch

When working on the `dev` branch, always document any notable changes in `docs/src/content/docs/changelog.md` under the `## [Unreleased]` heading:

1. **Always use `## [Unreleased]` on `dev`**: Place new entries under `## [Unreleased]` (grouped cleanly under `### Added`, `### Changed`, `### Fixed`, or `### Removed`). If `## [Unreleased]` does not exist at the top of `docs/src/content/docs/changelog.md` yet, create it immediately above the most recent published version heading.
2. **Never modify published release sections**: Do **not** append or edit entries under already-published version headings (such as `## [1.1]` or `## [1.0.0]`) while developing on `dev`.
3. **Promoting releases**: Only promote `## [Unreleased]` to a versioned heading (`## [<version>] — YYYY-MM-DD`) when preparing an actual release on a `release/<version>` branch (see `.agents/release/SKILL.md`).
