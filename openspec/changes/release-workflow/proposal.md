## Why

Cerbo needs a repeatable release path that can bump versions, build platform-specific installers, generate release notes, and publish everything to GitHub without hand-built ad hoc steps. This reduces release friction and makes tagged releases consistent across CLI and desktop builds.

## What Changes

- Add a manual GitHub Actions release workflow triggered by selecting a version bump level.
- Update version metadata before tagging so the repo state matches the published release.
- Create a release tag from the selected `major`, `minor`, or `patch` bump.
- Build and publish Linux artifacts: `tgz`, `AppImage`, `deb`, and `rpm`.
- Build and publish macOS `dmg` artifacts.
- Generate release notes from merged PRs in the tag range using an LLM-assisted summary.
- Publish a GitHub Release with short installation instructions for CLI and desktop users on Nix-based, Debian-based, Red Hat-based, and macOS systems.

## Capabilities

### New Capabilities
- `release-workflow`: manual release automation that bumps version, creates tags, builds release artifacts, generates notes, and publishes GitHub Releases.

### Modified Capabilities
- None.

## Impact

- GitHub Actions workflow configuration for release automation.
- Version source files used by Cerbo build outputs.
- Packaging and release publication for CLI and desktop artifacts.
- Release note generation and GitHub Release content.
