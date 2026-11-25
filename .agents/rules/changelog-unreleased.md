---
trigger: always_on
---

# Changelog Rule for the `dev` Branch

When working on the `dev` branch, always document any notable changes in `CHANGELOG.md` and `docs/src/content/docs/changelog.md` under the `## [Unreleased]` heading following the format [Keep a Changelog](https://keepachangelog.com/en/1.1.0/):

1. **Always use `## [Unreleased]` on `dev`**: Place new entries under `## [Unreleased]` (grouped cleanly under `### Added`, `### Changed`, `### Fixed`, or `### Removed`). If `## [Unreleased]` does not exist at the top of `docs/src/content/docs/changelog.md` yet, create it immediately above the most recent published version heading.
2. **Never modify published release sections**: Do **not** append or edit entries under already-published version headings (such as `## [1.1]` or `## [1.0.0]`) while developing on `dev`.
3. **Promoting releases**: Only promote `## [Unreleased]` to a versioned heading (`## [<version>] — YYYY-MM-DD`) in `docs/src/content/docs/changelog.md` or the format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) in `CHANGELOG.md`, when preparing an actual release on a `release/<version>` branch (see `.agents/release/SKILL.md`).
4. **The changelog is for the program only**: Only update the changelog with changes made to the program `lsi`. Do not include changes made to the git related files, .gitignore, to github workers, or development tools meant to maintain the program. Only include in the changelog updates that are made to the program `lsi`.
