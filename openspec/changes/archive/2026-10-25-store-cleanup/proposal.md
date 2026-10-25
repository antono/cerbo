## Why

The UUID object store works, but its write path can destroy user data. `page.md` — the
only copy of a note — is overwritten by a bare `fs::write` (`core/src/object.rs:528`,
`core/src/page.rs:90`) with no temp+rename, fsync, lock, trash, or backup anywhere. A full
disk or `kill -9` mid-save loses the note. Separately, a title containing `:type :Source`
permanently bricks its page, because `meta.ttl` is hand-rolled Turtle written unescaped and
parsed by substring match. And none of it is verifiable: `cargo test -p cerbo-core`
fails 4-8 of 77 at random, `cargo clippy --workspace` hard-fails, `nix build` skips tests.

Analysis: `docs/store-model-review.md` (18 confirmed findings).

## What Changes

- **Every vault mutation becomes atomic and durable** — an interrupted or failed write
  leaves the previous content intact. Enforced by CI, not convention.
- **`meta.ttl` becomes real, escaped Turtle**, so no title can inject a triple or truncate
  itself. This also un-bricks already-damaged pages with no file rewrite, and replaces the
  unmaintained `rio_turtle` dependency.
- **BREAKING: `.cerbo/index.json` removed.** Zero readers, already permanently stale,
  self-deletes on any parse error, O(N) per page create. Title resolution is respecified as
  what the code actually does — a `meta.ttl` scan.
- **BREAKING: `cerbo-migrate` deleted.** `docs/storage.md:155` already declares pre-UUID
  vaults incompatible and directs users to migrate manually, so the crate contradicts
  shipped policy. It is also the repo's most destructive code and the sole clippy blocker.
- **Delete becomes recoverable** via `.cerbo/trash/`.
- **Backrefs stop fabricating ghost objects** for nonexistent targets, and reindex converges
  in one batched pass.
- **Page titles come from one source** (`meta.ttl`), not two.
- **The test suite and lints actually run**, so everything above is verifiable.

## Non-goals

- `cerbo fsck`; a vault-wide lock (this narrows races, not eliminates them).
- `index --page` incrementality; per-save content history.
- The layout question (UUID dirs vs identity-in-frontmatter) and the symlink projection —
  deferred; see `docs/store-model-review.md` §6.
- Full-text search; auto-materializing the symlink tree.

## Capabilities

### New Capabilities

- `durable-writes`: the atomic write protocol every vault mutation must use, and its
  crash-safety guarantees.
- `object-trash`: recoverable deletion, trash layout, retention.

### Modified Capabilities

- `uuid-object-storage`: valid escaped Turtle with a real subject IRI; anchored type
  parsing; `index.json` dropped from the vault contract.
- `slug-resolution`: title→UUID comes from a `meta.ttl` scan, not `index.json`.
- `wikilink-editing`: resolution and autocomplete use that same scan.
- `vault-init`: `cerbo init` no longer creates `index.json`.
- `vault-management`: `cerbo init`'s stated vault contents drop `index.json`.
- `page-crud`: CRUD no longer touches `index.json`; delete moves to trash; titles come
  from `meta.ttl`.
- `backlinks`: never write a backref for a nonexistent target; one diff per save.
- `page-metadata-index`: reindex is a single batched pass, not clear-then-rebuild.

## Impact

- **Crates:** `core` (new `fsio.rs`, deleted `index.rs`, plus `object`/`page`/`links`/
  `annotations`/`metadata_index`); `cli`; `migrate/` dropped from workspace members.
- **Deps:** `+oxttl`, `+oxrdf`, `-rio_turtle`; `tempfile` optional → normal.
- **Vault format:** `index.json` disappears (ignored, not migrated); `meta.ttl` rewritten
  valid on next write; `page.md` never touched.
- **Docs:** `README.md`, `docs/storage.md`, `cli/man/cerbo.md`. **CI:** `doCheck = true`.
