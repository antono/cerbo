# Verification Record

Every scenario in `specs/durable-writes/spec.md` and `specs/object-trash/spec.md`, mapped to
the automated test that covers it or the manual result that records it. Task 10.6.

Automated tests run with `cargo test --workspace` (113 in `cerbo-core`, 21 in `cerbo`, 2
network tests marked `#[ignore]`). Manual results were produced on Linux 6.12.97, ext4 for
the crash test and a 64 KB tmpfs for the full-disk test.

## durable-writes

### Requirement: Atomic replacement of vault files

| Scenario | Covered by |
|---|---|
| Save interrupted by process termination | **Manual 10.3** below, plus `fsio::tests::failed_write_leaves_destination_intact` |
| Save fails because the filesystem is full | **Manual 10.4** below |
| Concurrent readers never observe a partial write | **Manual 10.3b** below |
| Temporary files do not leak into the vault view | `page::tests::page_list_skips_leftover_temp_files`, `fsio::tests::leaves_no_temp_file_behind`; the "subsequent write still succeeds" half is covered by Manual 10.3, where 140 writes succeeded with temp debris present |

### Requirement: Durability across power loss

| Scenario | Covered by |
|---|---|
| Power loss immediately after a reported-successful save | **Not testable here** — needs real power interruption or a fault-injecting block device. Verified by inspection: `fsio::write_atomic` calls `sync_data()` on the temp file before `persist`, and `sync_all()` on the parent directory after the rename, which is the documented protocol for making both the content and the new name durable. |
| Power loss during a save | Same. The rename is the only step that changes `page.md`, and it is atomic in the filesystem's journal, so a crash on either side of it leaves one complete version. |

### Requirement: Uniform application across all vault writes

| Scenario | Covered by |
|---|---|
| Metadata write is atomic | `ObjectMeta::write_to_file` routes through `fsio::write_atomic`; enforced mechanically — see the next row |
| Backreference write is atomic | `write_backrefs_to_path` routes through `fsio::write_atomic`; `links::tests::backrefs_ttl_is_valid_turtle_with_its_own_subject` exercises it |
| A new non-atomic write path is rejected | **Verified by deliberate reintroduction.** Adding `fs::write` to `core/src/links.rs` made both guards fail: `cargo clippy --workspace --all-targets -- -D warnings` errored with "use of a disallowed method `std::fs::write`" (via `core/clippy.toml`), and `scripts/check-atomic-writes.py` exited 1 naming the line. Both pass on a clean tree; the script also runs as the `atomic-writes` flake check. |

### Requirement: Write errors are reported, not swallowed

| Scenario | Covered by |
|---|---|
| Metadata update fails during a page save | `object::tests::a_failing_metadata_update_is_reported_not_swallowed` (the metadata stamp runs after `page.md` is written, so this is exactly the content-written-metadata-not case), plus `object::tests::an_unwritable_object_fails_the_save` |
| Backreference update fails during a page save | `links::tests::a_failing_backref_update_fails_the_save` — the error names the target UUID and the word "backreference" |

## object-trash

### Requirement: Delete moves objects to trash

| Scenario | Covered by |
|---|---|
| Delete a page | `object::tests::delete_moves_the_object_to_trash_intact` — asserts `page.md`, `meta.ttl` and an extra binary file all survive byte-for-byte |
| Deleted page disappears from listings | `object::tests::a_trashed_object_disappears_from_listings_and_does_not_resolve`, `vault_symlink::tests::trash_is_invisible_to_the_symlink_tree` |
| Deleted page no longer resolves | `object::tests::a_trashed_object_disappears_from_listings_and_does_not_resolve` |

### Requirement: Trash entries are uniquely named and identifiable

| Scenario | Covered by |
|---|---|
| Same object deleted, recreated, and deleted again | `object::tests::deleting_the_same_uuid_twice_keeps_both_entries` — asserts both entries exist with their own content |
| Trash entry identifies its object | `object::tests::delete_moves_the_object_to_trash_intact` — asserts the entry name contains the UUID and starts with a timestamp |

### Requirement: Trash is excluded from all vault operations

| Scenario | Covered by |
|---|---|
| Reindex ignores trash | `metadata_index::tests::reindex_ignores_the_trash` |
| Symlink rebuild ignores trash | `vault_symlink::tests::trash_is_invisible_to_the_symlink_tree` — asserts no leaf for the trashed object and that a second rebuild still succeeds, so the safe-wipe check does not treat trash as foreign |
| Links to a trashed object are broken, not resurrecting | `object::tests::saving_a_link_to_a_trashed_object_does_not_resurrect_it`, `links::tests::saving_a_link_to_a_missing_object_creates_nothing` |

### Requirement: Read-only objects cannot be deleted

| Scenario | Covered by |
|---|---|
| Delete a source-type object | `object::tests::deleting_a_read_only_object_creates_no_trash_entry` — asserts the error, that the object is untouched, and that no trash entry exists |

### Requirement: Trash retention is user-controlled

| Scenario | Covered by |
|---|---|
| Trash is never auto-pruned | No code path removes anything from `.cerbo/trash/`: `trash_dir` is written to only by `object_delete`, and nothing reads or deletes it. The reindex and symlink tests above both run with a non-empty trash and leave it intact. |
| Trash is excluded from version control by default | **Manual, below**: a fresh `cerbo init` writes `/cerbo/` and `/.cerbo/trash/` to `.gitignore`, and a second `init` adds neither line again |

## Manual results

### 10.3 — Crash test

A driver looping `object_write` on a 140 013-byte page was killed with `SIGKILL` at a random
point, 140 times, with the destination reset to a known "old" content before each run.

```
complete-old: 6   complete-new: 74   TORN: 0     (80 runs, kill after 0.02–1.2 ms)
complete-old: 0   complete-new: 60   TORN: 0     (60 runs, kill after 0.2–8 ms)
```

Every observation of `page.md` was one of the two complete versions; none was truncated,
empty or mixed. Interrupted runs left `.cerbo-tmp-*` siblings in the object directory (28
after one batch), and subsequent writes to the same destination continued to succeed.

### 10.3b — Concurrent readers

Two writers looping full 140 KB writes against one page while a reader hashed the same path
continuously for four seconds:

```
reads: 52715  torn/partial: 0
```

### 10.4 — Full-disk test

A 64 KB tmpfs (56 KB free) holding a page whose content was 34 bytes; a 140 013-byte save
was attempted:

```
exit=3
write reported error: Failed to write page.md: No space left on device (os error 28)
  at path ".../objects/137e0000-.../.cerbo-tmp-n4k3lQ"
RESULT: previous content survived intact
page.md now: 34 bytes
leftover temp files: 0
```

The error surfaced to the caller, the previous content was untouched, and the partial temp
file was removed on the way out.

### `.gitignore`

```
$ cerbo init      # fresh vault
$ cat .gitignore
/cerbo/
/.cerbo/trash/
$ cerbo init      # again
$ cat .gitignore
/cerbo/
/.cerbo/trash/
```

### 10.2 — `nix build` with `doCheck = true`

`nix/pkgs.nix` sets `doCheck = true`, and the CLI integration tests were made sandbox-safe
for it: the binary path now comes from `CARGO_BIN_EXE_cerbo` instead of a hard-coded absolute
path, and the two network-dependent tests are `#[ignore]`d.

The build initially could not run at all, for a reason unrelated to this change. crates.io
now returns **HTTP 403** to any request whose `User-Agent` begins with `curl/`, and nixpkgs'
`fetchurl` sends `curl/<version> Nixpkgs/<version>`, so `cargo-vendor-dir` failed on
pre-existing dependencies before any Cerbo code was compiled:

```
$ curl -sI -o /dev/null -w '%{http_code}\n' -A 'curl/8.21.0'            .../download   403
$ curl -sI -o /dev/null -w '%{http_code}\n' -A 'curl/8.21.0 Nixpkgs/25.11' .../download   403
$ curl -sI -o /dev/null -w '%{http_code}\n' -A 'Nixpkgs/25.11'          .../download   302
```

`NIX_CURL_FLAGS` (a `fetchurl` impure env var that could override the agent) does not help on
a multi-user install, because the daemon builds fixed-output derivations in its own
environment. The 481 missing crate tarballs were instead fetched from `static.crates.io` with
a valid agent and added with `nix-store --add-fixed sha256`, which places each at exactly the
path its derivation expects — all 481 matched, so nothing was patched or pinned to work
around it. This is an environment workaround, **not** a repository change; the underlying
problem is upstream and will need either a newer nixpkgs fetcher or a reachable mirror in CI.

With vendoring unblocked, the build ran the suite in the sandbox:

```
running 23 tests   (cli_integration)
test result: ok. 21 passed; 0 failed; 2 ignored
running 113 tests  (cerbo-core)
test result: ok. 113 passed; 0 failed; 0 ignored
```

It then failed in `postInstall` — a genuine bug this change is the first to reach, since
`doCheck = false` had never let the build get that far in this environment. The man-page glob
only looked in `target/release/build/cerbo-*/out`, but Cargo writes build-script output to
`target/<triple>/release/...` when a target is set; the script also used `exit 0` on success,
which would have aborted the remaining phases. Both are fixed in `nix/pkgs.nix`.

`nix build .#cerbo` then succeeded, with `cerbo.1.gz` installed and rendering, mentioning the
trash 8 times, documenting the non-zero exit, and no longer mentioning `index.json`.

`.#cerbo-desktop` initially could not be built either, for a second cause also unrelated to
this change. It depends on `cerbo-frontend`, which hung indefinitely in bun2nix's
`bunNodeModulesInstallPhase` — 28 sleeping threads, no open sockets, 0.71 s of CPU over
minutes, with no log output at all.

The cause was a **stale `nix/bun.nix`**. Commit `6c04dfa` bumped `@tauri-apps/cli` from
2.10.1 to 2.11.2 in `bun.lock`, but the Nix manifest was never regenerated — the step
`CLAUDE.md` calls for whenever `package.json` or `bun.lock` changes. Twelve packages had
drifted, the whole `@tauri-apps/cli` family:

```
in bun.lock but NOT in nix/bun.nix: 12   (@tauri-apps/cli* @ 2.11.2)
in nix/bun.nix but NOT in bun.lock: 12   (@tauri-apps/cli* @ 2.10.1)
```

So the offline cache lacked every 2.11.2 package, bun fell back to the network, and in the
sandbox — which has none — it blocked forever instead of failing. Running the same command
outside the sandbox, where the network exists, produced the errors that identified it by
name. Regenerating with the pinned bun2nix 2.1.0 (`bun2nix -o nix/bun.nix`) brought the
drift to zero across all 331 packages, and the frontend then built.

A red herring worth recording: the repro also showed `EACCES ... rename()` writing into the
bun cache, because the hook copies it from the read-only store with `cp -r`. That only bites
for packages bun has to download, so a correct manifest never reaches it. No change needed.

With both fixes in place, from a clean evaluation:

```
$ nix build .#cerbo .#cerbo-desktop
/nix/store/...-cerbo-0.1.0
/nix/store/...-cerbo-desktop-0.1.0
```

and both derivations ran the suite in the sandbox:

```
running 23 tests   (cli_integration)   → 21 passed, 0 failed, 2 ignored
running 113 tests  (cerbo-core)        → 113 passed, 0 failed
```

`cerbo` installs `bin/cerbo` and `share/man/man1/cerbo.1.gz`; `cerbo-desktop` installs
`bin/cerbo-desktop` and its `.desktop` entry. Task 10.2 is met.

Documentation edits made after this run change the flake source hash and so trigger a
rebuild, but touch no code; the build above is of the same source tree as the final state
apart from Markdown.

## Not verified here

- **Desktop end-to-end pass (task 10.5).** Needs a human at a GUI. The equivalents that can be
  driven headlessly all pass: create, edit, rename (including a quoted title), link, backlinks,
  delete-to-trash, reindex and `cerbo symlink` were exercised end to end through the `cerbo`
  CLI, and `page::tests::renaming_to_a_quoted_title_keeps_meta_valid` covers the rename dialog's
  core behaviour.
