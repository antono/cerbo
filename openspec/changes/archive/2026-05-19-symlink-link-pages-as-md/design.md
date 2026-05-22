## Context

`cerbo symlink` materialises a human-readable tree at `<vault-root>/cerbo/`. Each
leaf is currently a symlink named `<slug>` pointing to the object *directory*
`.cerbo/objects/<uuid>/`. Editors see an opaque directory, not a file, so they
cannot open it. Renaming the leaf to `<slug>.md` and redirecting the target to
`page.md` inside the directory gives editors a real, openable file. Attaching
`.md` lets editors (Obsidian, Zed, Neovim with filetype rules, etc.) associate
the extension with cerbo markdown independently of the `.md` extension used by
generated or template files.

The change is contained entirely within `core/src/vault_symlink.rs`. No new
dependencies are required.

## Goals / Non-Goals

**Goals:**
- Leaf symlinks named `<slug>.md`, pointing to `.cerbo/objects/<uuid>/page.md`
- Attachment objects (`ObjectType::Attachment`) excluded from the tree
- Existing symlink trees (directory targets, no extension) replaced cleanly on next rebuild

**Non-Goals:**
- Changing the `cerbo/` output directory name or location
- Changing the `virtualPath`-based directory hierarchy inside `cerbo/`
- Modifying `page.md` content or encoding
- Supporting Windows (symlink behaviour remains Unix-only as before)

## Decisions

### 1. Extension appended at path construction time, not stored in `PlanEntry.slug`

`PlanEntry.slug` stays extension-free. The `.md` suffix is appended where the
leaf `PathBuf` is constructed:

```
// Before
let leaf_path = leaf_parent.join(&entry.slug);

// After
let leaf_path = leaf_parent.join(format!("{}.md", &entry.slug));
```

`rendered_path()` (used for collision detection) must also append `.md` so
collision paths are canonical. Keeping the slug clean avoids polluting slug
logic and storage with a presentation detail.

**Alternative rejected**: Store `slug + ".md"` in `PlanEntry`. This would bleed
the extension into collision messages and any future code that reads the plan.

### 2. Exclude Attachment objects by type match, not by absence of `page.md`

Change the filter from skip-Ontology to allow-Product-or-Source only:

```rust
// Before
if matches!(meta.object_type, ObjectType::Ontology) { continue; }

// After
if !matches!(meta.object_type, ObjectType::Product | ObjectType::Source) { continue; }
```

This is explicit and survives future additions to `ObjectType` (new variants are
excluded by default rather than accidentally included).

**Alternative rejected**: Check for `page.md` existence at scan time. Fragile —
a corrupt or in-progress object directory should not silently become a symlink.

### 3. Target path is `<uuid>/page.md`, computed relative to leaf parent

```rust
// Before
let object_dir = vault_root.join(".cerbo").join("objects").join(&entry.uuid);
let rel_target  = pathdiff::diff_paths(&object_dir, &leaf_parent)?;

// After
let page_file  = vault_root.join(".cerbo").join("objects").join(&entry.uuid).join("page.md");
let rel_target = pathdiff::diff_paths(&page_file, &leaf_parent)?;
```

`pathdiff` handles depth correctly; no other change needed for portable targets.

### 4. Safe-wipe check unchanged in logic

`symlink_points_into(path, objects_root)` resolves the symlink target and checks
it is under `<vault-root>/.cerbo/objects/`. Old directory-targeting symlinks
resolve to `.cerbo/objects/<uuid>/` which is under `objects_root` — they pass.
New file-targeting symlinks resolve to `.cerbo/objects/<uuid>/page.md` which is
also under `objects_root` — they also pass. No code change required for the
safe-wipe check.

## Risks / Trade-offs

- **Broken symlink if `page.md` absent** — A Product/Source object without a
  `page.md` (e.g. partially written by an older cerbo version) yields a broken
  symlink. Mitigation: log a warning at scan time and skip objects where
  `page.md` does not exist, same pattern as the existing `meta.ttl` check.

- **Extension collision is impossible** — A slug `foo` and virtual path `foo`
  cannot collide with `foo.md` as a leaf, because virtual paths are
  directories and never carry the `.md` extension. No collision detection
  change needed beyond updating `rendered_path`.

- **Editor association** — `.md` is conventionally used for Mermaid diagrams.
  Users with Mermaid plugins may need to add a filetype override for `.md` to
  be treated as markdown. Trade-off is accepted: the user explicitly requested
  `.md` for editor support, and Mermaid/markdown coexistence is a user-side
  config concern.

## Migration Plan

No migration step required. On the next `cerbo symlink` run:

1. Safe-wipe check passes against the old tree (directory targets are under `objects_root`).
2. New tree is staged in `cerbo.tmp-<pid>/` with `.md` leaves → `page.md` targets.
3. Atomic rename replaces old tree.

Rollback: revert the code change and rerun `cerbo symlink`; the old tree is
rebuilt from `.cerbo/objects/` metadata.

## Open Questions

_(none)_
