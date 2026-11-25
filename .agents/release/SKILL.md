---
name: release
description: Create a new release, update documentation, version numbers and run tests and checks
---

# Release skill

A skill to create a new release update documentation, version numbers and run tests and checks before finishing the git flow release branch.
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# Trigger

This skill is triggered only when requested to create a new release. Any added functionalities are updated to a Unpublished release in the changelog file in the docs when working with the project in the dev branch.
This skill should be executed only when on a release branch.

# Core workflow

1. **Update README.md**: Update the README.md in the root folder of the project with the changes made to the program.
2. **Update docs/**: Update the docs with new functionalities, changes, if there is a new keybind changed update it. And update the Changelog page with the version number of the current git flow release (release/<version>): if the git flow release branch is release/1.1, than 1.1 is the new release number followed by a "—" (em dash) followed by today's date.
3. **Update CHANGELOG.md**: Update the CHANGELOG.md file at the root of the project following the format [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
4. **Run checks**: run cargo check and cargo clippy, test if the program compiles, and print recommended crates and library updates.

# > [!IMPORTANT]
>
> Do not run git flow start or finish commands, instead run git branch to print the branch name and extract the version number after the release/ prefix.
> If the current branch is not a git flow release branch, abort.
