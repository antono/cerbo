## Why

`cerbo symlink` currently creates leaf symlinks that point to the object directory (`.cerbo/objects/<uuid>/`), which editors cannot open as a file. Renaming the leaf to `<slug>.mmd` and targeting `page.md` directly gives editors a real file to open and a recognisable extension to associate with cerbo markdown.

## What Changes

- Leaf symlinks are renamed from `<slug>` to `<slug>.md`.
- Symlink targets change from the object directory (`.cerbo/objects/<uuid>/`) to the page file (`.cerbo/objects/<uuid>/page.md`).
- Only page-type objects (Product, Source) are materialised. Attachment objects are excluded in addition to the existing Ontology exclusion.
- The safe-wipe check is updated to validate symlinks that resolve to `page.md` files inside `.cerbo/objects/` (not directory targets).

## Capabilities

### New Capabilities

_(none)_

### Modified Capabilities

- `vault-symlink`: Symlink target form changes from object directory to `page.md`; leaf name gains `.mmd` extension; Attachment objects are now excluded from the tree.

## Impact

- `core/src/vault_symlink.rs` — target path construction, leaf name generation, object type filter
- `openspec/specs/vault-symlink/spec.md` — three requirements change: symlink target form, tree projection (leaf naming), skip-attachment rule
- `cli/man/cerbo.md` — update example output and description to reflect `.mmd` leaves
