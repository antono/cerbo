# Proposal

## Why

`cerbo import-vault` (change `add-vault-import`) imports an Obsidian or
Tolaria vault, but only from a terminal. Someone who just installed the
desktop app has no way in short of the CLI. A vault of 137 notes should be
importable with a few clicks, with the dry-run report shown before anything
is written.

## What Changes

- **New action `vault.import` ("Import vault…")** next to `vault.add`: in the
  command palette, the vault selector and the empty-state toolbar.
- **Import dialog, four steps:**
  1. *Source app:* two cards, **Obsidian first**, Tolaria second.
  2. *Folder:* native folder picker. The core `detect` runs on the choice; if
     it finds the other format (or none) the dialog says so and offers to
     switch, it never silently imports as the wrong format.
  3. *Target:* "Create a new vault" (default; folder + name) or one of the
     registered vaults.
  4. *Preview and import:* the dry-run report (pages, attachments, links
     rewritten, skipped, unresolved) and an **Import** button. A repeat
     import is refused with the core message plus an "Import again anyway"
     checkbox (`force`).
- **Two Tauri commands** over the existing core API: `vault_import_plan`
  (dry run) and `vault_import_run`. No new import logic.
- After success the new/target vault is opened.

## Capabilities

### New Capabilities

- `vault-import-ui`: the desktop import dialog and its commands.

### Modified Capabilities

- `vault-import`: the "Desktop UI" non-goal is lifted.

## Non-goals

- Re-sync of an earlier import, other formats, drag-and-drop of a folder.
- Per-note selection; the import is the whole folder, as in the CLI.
- A determinate progress bar (core has no progress callback; v1 shows a
  spinner and the counts from the plan).

## Impact

- **Frontend:** `src/lib/ImportVaultDialog.svelte`, `actions.ts`,
  `stores.svelte.ts`, toolbar/palette wiring.
- **Tauri:** `src-tauri/src/vault.rs` (+ handler registration).
- **Core/CLI:** none. Man page unchanged (CLI API untouched).
