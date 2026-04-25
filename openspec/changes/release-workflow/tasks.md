## 1. Workflow setup

- [ ] 1.1 Add a manual GitHub Actions release workflow triggered by version bump selection
- [ ] 1.2 Add workflow inputs for `major`, `minor`, and `patch` release bumps
- [ ] 1.3 Set explicit `timeout-minutes: 20` on every job

## 2. Version management

- [ ] 2.1 Compute the next semantic version from the latest tag and selected bump level
- [ ] 2.2 Update all version-bearing files before tagging
- [ ] 2.3 Commit and push the version bump before release publication

## 3. Build and package artifacts

- [ ] 3.1 Build the Linux `tgz` release artifact
- [ ] 3.2 Build the Linux `AppImage` artifact
- [ ] 3.3 Build the Linux `deb` artifact
- [ ] 3.4 Build the Linux `rpm` artifact
- [ ] 3.5 Build the macOS `dmg` artifact

## 4. Release publication

- [ ] 4.1 Generate release notes from merged PRs between the previous tag and new tag
- [ ] 4.2 Add short installation instructions for CLI and desktop users
- [ ] 4.3 Create or update the GitHub Release for the tag
- [ ] 4.4 Upload all build artifacts to the GitHub Release
