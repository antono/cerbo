# Tasks: Store Cleanup

Sequenced per `design.md` → Migration Plan. Sections 1–2 must land before anything else:
until the gates are green, no later fix is verifiable.

All `cargo` commands run inside `nix develop`.

## 1. Delete the migrate crate

- [x] 1.1 Delete the `migrate/` directory and remove `"migrate"` from `Cargo.toml` workspace members; verify `cargo metadata --no-deps` no longer lists `cerbo-migrate`
- [x] 1.2 Remove the migration section from `README.md:98-108`; verify `grep -rn "cerbo-migrate" README.md docs/ cli/man/` returns nothing
- [x] 1.3 Verify `cargo clippy --workspace --all-targets -- -D warnings` no longer fails on the `Regex::new(r"(?!)")` at the old `migrate/src/main.rs:295`

## 2. Make the test suite and lints run green

- [x] 2.1 Replace the fixed temp path in `core/src/object.rs:592` (`temp_dir()/cerbo_test`) with a per-test `TempDir`, returning it from the fixture so it outlives the test; verify `cargo test -p cerbo-core object::` passes
- [x] 2.2 Replace the fixed temp path in `core/src/links.rs:200` (`temp_dir()/cerbo_test_links`) the same way; verify `cargo test -p cerbo-core links::` passes
- [x] 2.3 Delete the dead `[test] threads = 1` stanza from **both** `.cargo/config.toml` and `core/.cargo/config.toml` (`[test]` is not a Cargo config table and has never had effect); verify `cargo test -p cerbo-core` passes 77/77 on three consecutive runs with default parallelism
- [x] 2.4 Change `test-core.sh:1` to `#!/usr/bin/env bash`; verify `./test-core.sh` runs on NixOS without a "no such file or directory" error
- [x] 2.5 Verify `cargo test --workspace` passes and `cargo clippy --workspace --all-targets -- -D warnings` is clean
- [x] 2.6 Set `doCheck = true` in `nix/pkgs.nix:13`; verify `nix build .#cerbo .#cerbo-desktop` succeeds with tests running

## 3. Atomic durable writes

- [x] 3.1 Add `tempfile` as a normal (non-optional) dependency of `core`, keeping the `test-utils` feature working; verify `cargo build -p cerbo-core` succeeds
- [x] 3.2 Create `core/src/fsio.rs` with `write_atomic(dest, bytes)` per `design.md` D1 (temp file in destination's directory → `sync_data()` → `persist` → parent-dir `sync_all()`), and register the module in `core/src/lib.rs`; verify a unit test writes a file and reads back identical bytes
- [x] 3.3 Preserve the destination's file mode onto the temp file before `persist`; verify a test asserts mode is unchanged after rewriting an existing file with mode `0o600`
- [x] 3.4 Add a unit test proving the destination retains its previous content when the write fails mid-way (inject failure via an unwritable temp target or a short write); verify the old content is intact and an `Err` is returned
- [x] 3.5 Route `core/src/object.rs:528` (`object_write`) and `core/src/page.rs:90` (`page_update_title`) through `write_atomic` — the two paths that can destroy a note; verify existing page write/read tests still pass
- [x] 3.6 Route the remaining `core` writers through `write_atomic`: `object.rs:60,244,300,370,410,464,557`, `links.rs:140`, `annotations.rs:95`, and the four existing hand-rolled tmp+rename sites (`config.rs:31-34`, `state.rs:35-38`, `ui_settings.rs:52-54`, `vault.rs:60-63`); verify `cargo test --workspace` passes
- [x] 3.7 Give temp files a known prefix and make `page_list`, `vault_symlink::build_plan`, and `metadata_index` scans skip it; verify a test placing a leftover temp file in an object directory does not make it appear in `page_list` output
- [x] 3.8 Add a CI check failing the build on `fs::write`/`File::create` under `core/src` outside `fsio.rs` (grep or clippy `disallowed_methods` — decide per `design.md` Open Questions); verify the check fails when a `fs::write` is deliberately reintroduced and passes on a clean tree

## 4. Real Turtle for metadata, backrefs and annotations

- [x] 4.1 Add `oxttl = "=0.2.4"` and `oxrdf = "=0.3.4"` and remove `rio_turtle` from `core/Cargo.toml`; verify `cargo build -p cerbo-core` succeeds and `grep -rn "rio_turtle" core/` returns nothing
- [x] 4.2 Rewrite `ObjectMeta::to_turtle` (`object.rs:78-110`) to build `oxrdf` triples and serialise via `oxttl`, taking the object's UUID as a parameter so the subject is the real `<cerbo://objects/{uuid}>`, and declaring the `cerbo:` prefix; verify the output parses with an `oxttl` parser in a test
- [x] 4.3 Rewrite `ObjectMeta::from_turtle` (`object.rs:125-158`) to match on parsed predicate IRIs instead of `line.contains(...)`; verify all existing `object::` tests pass
- [x] 4.4 Add the regression test for the brick defect: a page titled `RDF :type :Source notes` round-trips with its correct type and full title, and remains writable and deletable; verify the test fails against the old parser and passes against the new one
- [x] 4.5 Add escaping round-trip tests for `He said "hi"`, a title containing a newline, and `C:\temp — заметки 🎉`; verify each reads back byte-identical to what was written
- [x] 4.6 Add a byte-stability test: read an object's metadata and write it back with no field changed; verify the file is byte-identical
- [x] 4.7 Add a recovery test proving an on-disk `meta.ttl` already damaged by a `:type`-injected title reads correctly after this change with no file rewrite; verify no write occurs during the read
- [x] 4.8 Rewrite `write_backrefs_to_path` (`links.rs:112-140`) to serialise via `oxttl` with the owning object's own subject IRI, routed through `write_atomic`; verify a test parses the output and that an object with no backlinks yields an empty backlink list
- [x] 4.9 Rewrite `annotations_write_to_path` (`annotations.rs:66-95`) to serialise via `oxttl`, removing the replace-all terminator bug at `:81-86`; verify a test with three annotations produces a document that parses and preserves each blank node's predicates
- [x] 4.10 Update the test fixture subject at `object.rs:803` and any other literal `<cerbo://objects/<uuid>>` occurrences; verify `grep -rn 'objects/<uuid>' core/ src-tauri/` returns nothing

## 5. Remove index.json

- [x] 5.1 Delete `core/src/index.rs` and its `mod` registration; verify `cargo build -p cerbo-core` fails only at the known call sites
- [x] 5.2 Remove the five `let _ = index::index_add(...)` call sites (`object.rs:249,305,374,414,855`) and the `fixtures.rs:34,57` uses; verify `cargo build --workspace` succeeds
- [x] 5.3 Stop creating `.cerbo/index.json` in `cerbo init` (`cli/src/main.rs`); verify a fresh `cerbo init` in a temp directory produces no `index.json`
- [x] 5.4 Verify `cerbo init` on a vault that already contains a legacy `index.json` succeeds and leaves the file untouched
- [x] 5.5 Verify page create, rename, delete, link resolution, and autocomplete all still work end to end with no `index.json` present — exercise via `cargo run -p cerbo -- page create/list/resolve` and the desktop app

## 6. Backref correctness

- [x] 6.1 Make `write_backrefs_to_path` refuse to `create_dir_all` a missing object directory (`links.rs:112-114`), reusing the existence guard at `metadata_index.rs:96-101`; verify saving a page containing `cerbo://objects/deadbeef-0000-0000-0000-000000000137` creates no directory and the save still succeeds
- [x] 6.2 Validate the extracted link id with `Uuid::parse_str` before it is used as a path segment; verify a test with a malformed id returns a broken-link result and touches no filesystem path
- [x] 6.3 Delete the duplicate link diff at `links.rs:179-188` (`object_write` already calls `update_backrefs` at `object.rs:535`); verify a test asserts the target's `backrefs.ttl` is written exactly once per save
- [x] 6.4 Propagate the previously discarded results: `update_backrefs` at `object.rs:535` and `let _ = backrefs_remove` at `object.rs:573`; verify a failing backref update makes the save return an error naming the failed part
- [x] 6.5 Rewrite `metadata_index::index_vault` to accumulate `HashMap<target, BTreeSet<source>>` in a single scan and write each target once, deleting the Phase-2 clear-all (`metadata_index.rs:45-53`); verify a full reindex of a vault where 137 pages link to one hub writes the hub's file once with all 137 backreferences
- [x] 6.6 Verify an interrupted full reindex leaves every object's `backrefs.ttl` holding either its previous or its newly computed content, never cleared
- [x] 6.7 Make `cerbo index` exit non-zero when it encounters an unreadable or unwritable object; verify the exit status and stderr naming the UUID

## 7. Recoverable delete

- [x] 7.1 Change `object_delete` (`object.rs:311-332`) to rename the object directory into `.cerbo/trash/<timestamp>-<uuid>/` instead of `remove_dir_all`; verify a test deletes a page and finds `page.md` and `meta.ttl` preserved byte-for-byte under the trash
- [x] 7.2 Verify the read-only refusal still happens before anything is moved: deleting a `:Source` object errors and creates no trash entry
- [x] 7.3 Verify deleting the same UUID twice (delete, recreate, delete) produces two distinct trash entries with neither overwriting the other
- [x] 7.4 Exclude `.cerbo/trash/` from `page_list`, link resolution, `metadata_index` scans, and `vault_symlink::build_plan`; verify a reindex and a symlink rebuild with a non-empty trash report no problems and create no symlinks for trashed objects
- [x] 7.5 Verify the safe-wipe check in `vault_symlink.rs:284-330` does not treat `.cerbo/trash/` as unsafe content and that `cerbo symlink` still succeeds
- [x] 7.6 Verify a trashed object is not resurrected: saving a page that still links to it creates nothing under `.cerbo/objects/` and the link reports as broken
- [x] 7.7 Add `/.cerbo/trash/` to the `.gitignore` written by `cerbo init`, appending without reordering existing content; verify a fresh init contains both `/cerbo/` and `/.cerbo/trash/`, and that re-running init does not duplicate either line

## 8. Page titles from a single source

- [x] 8.1 Change `page_list` (`core/src/page.rs:122-137`) to read the title from `meta.ttl` via `ObjectMeta::read_from_file` — as `vault.rs:130` already does — and delete the `// TODO: Parse meta.ttl properly` comment; verify `page.md` is no longer read during a listing
- [x] 8.2 Verify a page whose body H1 was changed outside Cerbo lists under the title recorded in `meta.ttl`
- [x] 8.3 Verify renaming a page to `He said "hi"` via the desktop rename dialog leaves `meta.ttl` valid and shows the exact title in the sidebar

## 9. Documentation

- [x] 9.1 Rewrite `docs/storage.md` to document the symlink projection layer (currently absent) and the trash area, and remove the `index.json` description; verify the described on-disk shape matches a freshly initialised vault
- [x] 9.2 Update `cli/man/cerbo.md` for the removed `index.json`, the new trash behaviour of `page delete`, and `cerbo index`'s non-zero exit; verify `nix build .#cerbo` regenerates `cerbo.1` and `man ./cerbo.1` renders
- [x] 9.3 Correct the `cerbo index --page` description at `cli/man/cerbo.md:127-137`, which currently advertises it for post-crash and external-edit repair — cases where it cannot converge; verify the text now directs users to a full-vault reindex
- [x] 9.4 Retire the five spec files describing deleted machinery (`rename-cascade`, `smart-rename-cascade`, `dynamic-slug-generation`, `virtual-path-input`, and the stale parts of `slug-resolution`), reconcile the `backrefs.ttl` contradiction between `uuid-object-storage/spec.md:88-98` and `backlinks/spec.md:51-58`, and resolve the `TBD` at `uuid-object-storage/spec.md:14`; verify `openspec validate --changes --strict` passes

## 10. Verification

- [x] 10.1 Verify `cargo test --workspace` passes on three consecutive runs with default parallelism, and `cargo clippy --workspace --all-targets -- -D warnings` is clean
- [x] 10.2 Verify `nix build .#cerbo .#cerbo-desktop` succeeds with `doCheck = true`
- [x] 10.3 Manual crash test: start a save of a 137 KB page, kill the process mid-write, and verify `page.md` still holds its complete previous content
- [x] 10.4 Manual full-disk test: save a page onto a filesystem with less free space than the page, and verify the error surfaces and the previous content survives
- [ ] 10.5 Manual end-to-end pass in the desktop app: create, edit, rename (including a title with a quote), link, follow a link, view backlinks, delete, then `cerbo symlink` and confirm the tree matches
- [x] 10.6 Verify every scenario in `specs/durable-writes/spec.md` and `specs/object-trash/spec.md` has a corresponding automated test or a recorded manual result
