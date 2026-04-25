## ADDED Requirements

### Requirement: Manual release dispatch
The system SHALL provide a manual GitHub Actions release workflow that starts from a user-selected semantic version bump level.

#### Scenario: User starts a release
- **WHEN** a maintainer manually triggers the workflow and selects `major`, `minor`, or `patch`
- **THEN** the workflow SHALL compute the next semantic version from the latest tag and the selected bump level

### Requirement: Version sync before tagging
The system SHALL update all release version metadata before creating the release tag.

#### Scenario: Version metadata is updated
- **WHEN** the workflow prepares a new release
- **THEN** the workflow SHALL update the repository version files so the source tree matches the release version

### Requirement: Linux release artifacts
The system SHALL build Linux release artifacts for `tgz`, `AppImage`, `deb`, and `rpm`.

#### Scenario: Linux artifacts are built
- **WHEN** the workflow runs on a Linux runner for a release tag
- **THEN** the workflow SHALL produce `tgz`, `AppImage`, `deb`, and `rpm` artifacts for upload

### Requirement: macOS release artifact
The system SHALL build a macOS `dmg` release artifact.

#### Scenario: macOS artifact is built
- **WHEN** the workflow runs on a macOS runner for a release tag
- **THEN** the workflow SHALL produce a `dmg` artifact for upload

### Requirement: PR-based release notes
The system SHALL generate release notes from merged pull requests between the previous tag and the new tag.

#### Scenario: Release notes are generated
- **WHEN** the workflow publishes a release
- **THEN** the workflow SHALL summarize merged PRs in the tag range into short release notes

### Requirement: Publish GitHub release assets
The system SHALL publish a GitHub Release for the new tag and upload all built artifacts to it.

#### Scenario: Release is published
- **WHEN** all build jobs complete successfully
- **THEN** the workflow SHALL create or update the GitHub Release for the tag and upload all artifacts

### Requirement: Installation instructions included
The system SHALL include concise installation instructions for CLI and desktop users in the GitHub Release body.

#### Scenario: Instructions are shown
- **WHEN** the GitHub Release is created
- **THEN** the release body SHALL include installation instructions for Nix-based systems, Debian-based systems, Red Hat-based systems, and macOS
- **AND WHEN** the instructions are displayed
- **THEN** they SHALL distinguish between the `cerbo` CLI and `cerbo-desktop` application

### Requirement: Bounded release jobs
The system SHALL enforce a 20 minute timeout on each release job.

#### Scenario: Release job hangs
- **WHEN** any job in the release workflow exceeds 20 minutes
- **THEN** GitHub Actions SHALL terminate the job
