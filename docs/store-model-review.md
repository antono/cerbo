# Store Model Review — UUID Object Store

**Date:** 2026-09-04
**Scope:** the UUID object store as merged into `main` (`globry-to-robots` #1 `6e16e87`,
`glory-to-robots` #2 `1efb315`)
**Method:** multi-agent code review, every finding put through an adversarial
refutation pass; the criticals re-verified by hand against the code.
**Result:** 20 findings raised, 18 confirmed, 2 refuted.

> **In plain terms:** Cerbo now gives every note a permanent random ID and stores it
> in a hidden folder named after that ID, so renaming a note can no longer break
> links to it. That core idea works and is shipped. The plumbing around it is
> unfinished: saving a note overwrites the only copy with no safety net, two Cerbo
> processes can silently overwrite each other, and the "semantic" metadata files it
> writes are not actually valid semantic-web files.

---

## 1. What shipped

Net over `28b5329..main`: **+5850 / −1867 across 42 files**. `core/src/rename.rs`
(291 lines) was deleted outright — rename-cascade became unnecessary once identity
moved from the slug to a UUID. That deletion is the clearest evidence the core
premise was correct.

### On-disk shape (as implemented)

```
<vault>/
├── .gitignore                      # "/cerbo/" — written only by `cerbo init`
├── .cerbo/
│   ├── index.json                  # title_to_uuid + uuid_to_path
│   ├── ontology-map.json
│   └── objects/<uuid>/
│       ├── page.md                 # content
│       ├── meta.ttl                # 8 scalar fields, hand-rolled Turtle
│       ├── backrefs.ttl            # inbound :hasBacklink only
│       └── annotations.ttl         # write-only
└── cerbo/<virtualPath>/<slug>.md   # relative symlinks -> ../.cerbo/objects/<uuid>/page.md
```

Two things worth stating plainly because neither is written down anywhere:

- Every content file in the vault is named `page.md`. The human-readable name exists
  only as a symlink leaf under `cerbo/`.
- `docs/storage.md` does not mention the symlink layer at all — 573 lines of
  data-destructive-if-wrong behaviour with no design doc behind it.

### Module ownership

| Concern | Owner | State |
|---|---|---|
| Object CRUD, `meta.ttl` | `core/src/object.rs` (903) | Works; 5 near-identical create bodies (`:205,:255,:336,:381,:814`) |
| Symlink projection | `core/src/vault_symlink.rs` (573) | Best-engineered code in the repo; **one** non-test caller (`cli/src/main.rs:719`) |
| Backlink cache | `core/src/links.rs` (321) | Works; double-writes per save; O(k²) per hub |
| Batch reindex | `core/src/metadata_index.rs` (378) | Full-vault works; `--page` cannot converge |
| Title→UUID index | `core/src/index.rs` (94) | Effectively dead — see §4.3 |
| Vault registry / discovery | `vault.rs` (786), `config.rs`, `state.rs` | Works; correctly uses tmp+rename |
| Annotations | `core/src/annotations.rs` (215) | Write-only; `annotations_read` returns `Ok(vec![])` (`:99-111`) |
| Legacy migration | `migrate/` | Destructive; panicking error path; output core cannot read |
| Desktop FS watcher | `src-tauri/src/index.rs:43-53` | Stub returning `Ok(())` |

### Test and gate reality

102 tests: 77 core, 23 CLI, 2 migrate, **0 in `src-tauri`** — the entire desktop
entry surface onto the new store is untested.

- `cargo test -p cerbo-core` fails **non-deterministically** — measured 4, 6, 7 and 8
  failures of 77 across four consecutive runs. Tests within a module share one fixed temp
  directory (`core/src/object.rs:592` uses `temp_dir()/cerbo_test`, `core/src/links.rs:200`
  uses `temp_dir()/cerbo_test_links`) and race each other. The `[test] threads = 1` config
  meant to prevent this exists in *both* `.cargo/config.toml` and `core/.cargo/config.toml`,
  and is silently ignored — `[test]` is not a Cargo config table.
- `test-core.sh:1` is `#!/bin/bash`, which does not exist on NixOS.
- `nix build` sets `doCheck = false` (`nix/pkgs.nix:13`).
- `cargo clippy --workspace --all-targets` hard-fails on `migrate/src/main.rs:295`.

**There is currently no documented command that runs the core suite green.** Nothing
below can be verified until that is fixed.

Zero coverage for: crash safety, concurrency, symlink repair, Turtle conformance,
end-to-end migration.

---

## 2. What the design got right

Several of these decisions are better than the code that implements them, and should
survive any cleanup.

- **The diagnosis.** "Slugs couple identity to presentation"
  (`openspec/changes/archive/2026-05-18-uuid-object-storage/proposal.md:3`) is the
  correct read. UUID-stable identity eliminates the whole rename-cascade class.
- **The design docs argue rather than assert.** UUIDv4 vs ULID/Nanoid/content-hash;
  Turtle vs JSON-LD/RDF-XML; HackMD spans vs double-braces vs frontmatter;
  JSON-now/Oxigraph-later — each with explicitly rejected alternatives and stated
  rationale (`design.md:61-237`).
- **`cerbo symlink` is genuinely well engineered.** Plan → validate → safe-wipe →
  stage in `cerbo.tmp-<pid>` → two-rename swap → crash-sweep
  (`vault_symlink.rs:84-169`). The recursive safe-wipe check refuses to touch
  anything that is not a directory or a symlink canonicalising inside this vault's
  own `.cerbo/objects/` (`:284-330`). Relative targets keep the vault movable, with a
  test that renames the vault root (`:462-477`). Full-rebuild-only is the right call
  — it gives GC for free.
- **Atomic-write discipline already exists in the crate** and is correct where
  applied: `config.rs:31-34`, `state.rs:35-38`, `ui_settings.rs:52-54`,
  `vault.rs:60-63`. The defect in §3.1 is that it was never extended to user content.
- **Refusing to keep `slug` as an alias for `uuid`** to shrink the desktop diff —
  "it perpetuates the wrong mental model" (`adapt-desktop design.md:63`).
- **Git-style vault discovery**: walk up for `.cerbo/`, stop at a mount boundary,
  exact error string (`vault.rs:517-538`).
- **Keeping the `.ttl` files as source of truth so any future graph DB stays a
  rebuildable cache** (`design.md:590`) is the right architectural instinct, even
  though the code only half-honours it.

---

## 3. Critical defects

### 3.1 Saving a note can destroy it

`page.md` — the only copy of a user's note — is overwritten by a bare `fs::write`:

- `core/src/object.rs:528` (`object_write`), immediately after reading `old_content`
  at `:526`
- `core/src/page.rs:90` (`page_update_title`)

No temp+rename, no `fsync`, no lock, and no trash, backup, snapshot, or git backing
anywhere in the workspace (`grep -rE 'sync_all|sync_data|flock|fs2|fd-lock'` over
`core/ cli/ src-tauri/ migrate/` returns nothing).

**Failure:** a 137 KB page saved onto a filesystem with 40 KB free. `fs::write`
truncates to zero, writes 40 KB, returns `ENOSPC`. The UI reports "save failed" and
the previous version is gone. Same for `kill -9` or power loss.

**Reachable from ordinary typing:** the debounced autosave path is
`PageEditor.svelte:239-243` → `stores.svelte.ts:343` → `src-tauri/src/page.rs:26` →
`core/src/page.rs:52` → `links.rs:173` → `object.rs:528`.

**Silent aftermath:** a truncated `page.md` is indistinguishable from a short page.
`page_list` still lists it, and the next save reads the truncated text as
`old_content` (`object.rs:526`), silently dropping every backref the lost text
carried.

### 3.2 A page title can permanently brick the page

Titles are interpolated unescaped into `meta.ttl`:

```rust
// core/src/object.rs:84
format!("    :title \"{}\" ;", self.title),
```

and parsed back by an `else if` chain whose **first** arm is a substring test:

```rust
// core/src/object.rs:126-137
if line.contains(":type :") {
    if line.contains(":Product") { ... }
    else if line.contains(":Source") { ... }
    ...
} else if line.contains(":title ") {
```

A title containing `:type :Source` hijacks the type arm and suppresses the `:title`
arm.

**Failure:** a page titled `RDF :type :Source notes` parses as `ObjectType::Source`
forever. `object_write` fails with "Cannot write to source type (read-only)"
(`object.rs:520`); `object_delete` fails the same way (`object.rs:323`). No CLI or
Tauri command can change an object's type, and `page_update_title` round-trips the
parsed type and re-emits `:type :Source` (`page.rs:66-71`). The page is permanently
unwritable and undeletable, and its stored title is erased.

The `:Ontology` variant is quieter and worse: such objects are filtered out of
indexing (`metadata_index.rs:31-40`) and skipped by the symlink plan
(`vault_symlink.rs:196-198`), so the page vanishes from both views while its files
sit intact on disk.

**This is read-side only.** `to_turtle` writes `:type :Product ;` on its own line
(`object.rs:83`) with the injected title on `:84`. Anchoring `from_turtle` on
`line.starts_with(":type")` un-bricks every already-damaged page on disk with **no
file rewrite and no migration**.

### 3.3 `index.json` is an unlocked, self-destructing, O(N²) file with no readers

`index_add` is load → `HashMap::insert` → rewrite the whole file
(`core/src/index.rs:57-66`), with a bare truncating `fs::write` (`:51`) and no lock.
On any parse failure `index_load` **deletes the file** and returns `Ok(default)`
(`:37-41`) — one bad byte is permanent total loss, and no rebuild path exists.

**Failure:** two processes create pages concurrently; both read 137 entries, both
insert, one entry silently vanishes — every call site discards the result
(`let _ = index::index_add(...)` at `object.rs:249,305,374,414,855`). Or process B
reads while A has truncated but not refilled: B deletes the index and writes one
entry, and 137 mappings are gone with `Ok(())` returned.

**Scale:** importing 100k pages rewrites the whole file 100k times — roughly 789 GB
of `write()` traffic, and ~93 ms of CPU per single create at 100k entries, paid on
every desktop "new page" click.

**The mitigating fact that decides the fix:** `index_resolve_title` (`index.rs:84`),
`index_resolve_uuid` (`:91`) and `index_remove` (`:70`) have **zero callers anywhere**
in `.rs`/`.ts`/`.svelte` — verified. `object_delete` never removes entries,
`page_update_title` never updates them, so the map is already permanently stale.
`uuid_to_path`'s value is `format!("objects/{}/", uuid)` — derivable from its own key
(`:62-63`). The only external reader is `migrate/src/main.rs:358` (its verify pass).

**Deleting it is a pure win.**

### 3.4 `page_list` reads every page body to find a title it already has

```rust
// core/src/page.rs:122-137
let title = if meta_path.exists() {
    // TODO: Parse meta.ttl properly
    // For now, try to extract from page.md
    std::fs::read_to_string(&page_md) ... find(|l| l.trim().starts_with("# "))
```

It checks that `meta.ttl` exists and then reads `page.md` anyway. ~400 MB read at
100k pages, on a synchronous `#[tauri::command]` (`src-tauri/src/page.rs:70-75`) that
`updatePageTitle` calls unconditionally (`stores.svelte.ts:382`).

It also creates two competing title sources — `meta.ttl`'s `:title` and the body H1 —
which diverge permanently under external editing. `vault.rs:130` already does this
correctly via `ObjectMeta::read_from_file`.

---

## 4. High-severity defects

### 4.1 Concurrent saves silently lose backlinks

`backrefs_add`/`backrefs_remove` are unsynchronized read-modify-write
(`links.rs:52-80`), and `write_backrefs_to_path` rewrites the whole file with a bare
`fs::write` (`links.rs:112-140`) — not even tmp+rename, unlike every other writer in
the crate. The window is doubled because `object_write` already calls
`update_backrefs` (`object.rs:535`) and then `page_write_with_links` re-runs the
identical diff (`links.rs:179-188`).

**Failure:** pages A and B both add a link to C. Desktop autosave for A and
`cerbo page write B` overlap; both read C's `[X]`; one writes `[X,A]`, the other
`[X,B]`. Later write wins, one backlink is gone. Recoverable only by a full
`cerbo index`. Severity is capped because `page.md` remains source of truth.

### 4.2 Stale backrefs are unfixable by the command documented to fix them

`index_page` iterates only the source page's **current** links
(`metadata_index.rs:90-112`) and does remove-then-add per current target. With no
outbound link record on the source side, it can add a missing backref but can never
strip one from a target it no longer links to.

Triggers, in decreasing likelihood: swallowed errors (`update_backrefs`'s result
discarded at `object.rs:535`; `let _ = backrefs_remove` at `:573`), external edits
through the symlink (nothing reindexes — the watcher is a stub at
`src-tauri/src/index.rs:43-53`, and `cerbo watch` does not exist), and a crash
between `:528` and `:535`.

**Failure:** A drops its link to B and saves. B's `backrefs.ttl` still lists A, and
the sidebar renders it as a live backlink. `cerbo index --page <A>` finds zero links,
touches nothing, reports success.

**Aggravating:** `cli/man/cerbo.md:127-137` advertises `--page` as the mode to use
"after manually editing page.md files outside of Cerbo" and "to verify metadata
integrity after crashes" — precisely the cases where it silently fails to converge.
Even a full reindex leaves stale backrefs on Ontology objects, which Phase 2 never
clears (`metadata_index.rs:31-39`).

### 4.3 A typo'd link fabricates permanent ghost object directories

`write_backrefs_to_path` calls `fs::create_dir_all(obj_dir)` unconditionally with no
existence check (`links.rs:112-114`), and `page_write_with_links` calls
`backrefs_add` for **every** link on **every** save (`links.rs:180`) — so the ghost is
recreated even if the user deletes it.

**Failure:** paste `[B](cerbo://objects/deadbeef-…)` and save.
`.cerbo/objects/deadbeef-…/backrefs.ttl` now exists. It is invisible to `page_list`
(needs `page.md`), to `build_plan` (needs `meta.ttl`, `vault_symlink.rs:187-190`), and
to `list_vault_objects` — and no command deletes it. But `cerbo resolve deadbeef-…`
now succeeds, because it is a bare `exists()` check (`cli/src/main.rs:580-587`).

**Worse:** `object_write` on a ghost passes the existence gate (`object.rs:511`) and
then **skips the read-only gate entirely**, because that gate is
`if meta_path.exists()` (`:516`). The same mechanism resurrects a deleted object:
after `page delete C`, the next save of any page still linking to C recreates
`objects/C/`.

### 4.4 No Turtle escaping, and the `.ttl` files are not valid Turtle at all

Verified against the real `ObjectMeta` API:

| Title written | Reads back as |
|---|---|
| `He said "hi"` | `He said ` (silent truncation, byte-unstable) |
| title containing `\n` | `` (erased) |

Reachable from the desktop rename dialog (`RenamePageDialog.svelte:21` →
`stores.svelte.ts:379` → `page.rs:62`) with no validation anywhere. `page.md`'s H1
keeps the real title, so the two diverge permanently. And `to_turtle` regenerates the
whole file from the struct, so any hand-added triple — or an invalid stored
`slug`/`virtualPath` — is erased on the next rename.

Independently, **every `.ttl` the code writes is unparseable Turtle**:

- The subject is the literal, never-substituted string `<cerbo://objects/<uuid>>` in
  all writers — `object.rs:82`, `links.rs:122`, `annotations.rs:69`,
  `vault_symlink.rs:394`, and the test fixture at `object.rs:803`. Angle brackets
  cannot nest in an IRI; and if they could, all N objects would share one subject.
- `cerbo:` is used at `object.rs:104,:108` but never declared — `:78-80` declares only
  `:`, `schema:`, `xsd:`.
- `object.rs:90` terminates the statement with `.` before four more subject-less
  predicate lines.

`rio_turtle = "0.8"` sits at `core/Cargo.toml:19` with **zero** `use` sites.
`grep -rE 'sparql|triplestore|Dataset|NamedGraph'` across the workspace returns
nothing. Note that `migrate/src/main.rs:315` gets the subject right with `format!` —
strong evidence the core writers are an oversight, not a choice.

### 4.5 Terminator fixup via `String::replace` corrupts annotations

```rust
// core/src/annotations.rs:77-86
lines.push(format!(
    "    :annotation [ :concept \"{}\" ; :type <{}> ; :position \"{},{}\" ] ;", ...));
if i == annotations.len() - 1 {
    *last = last.replace(" ;", " .");   // replace-all
}
```

The line carries three ` ;` separators, so the blank node's internal separators all
become ` .`. The corruption is invisible only because `annotations_read` is a stub.
`migrate/src/main.rs:317-320` already does this correctly by index.

(The same pattern at `links.rs:134` is benign today — those lines carry exactly one
` ;` each — but it is the same latent trap.)

### 4.6 Vault content is invisible to `ripgrep` by default

Verified: with `.gitignore` exactly as `cerbo init` writes it (`cli/src/main.rs:226-243`),
`rg "borrow checker" .` exits 1. Content lives under a dot-directory
(`.cerbo/objects/`), and the human-named view is symlinks that `rg`, `fd`, and
`find -type f` do not follow. The `/cerbo/` ignore entry defeats the `rg -L`
workaround. No `cerbo search` exists; full-text search is an explicit non-goal
(`design.md:54`).

### 4.7 No adoption path, and file-manager edits are silently reverted

Nothing turns an existing local `.md` into a note: `page create` takes a title only
(`cli/src/main.rs:189-235`), and `import` is URL-only via a `curl` shell-out
(`object.rs:483-498`). Conversely, a file-manager rename of a symlink leaf is silently
reverted (`vault_symlink.rs:296-330` classifies by file type only), and a foreign
regular file placed in `cerbo/` aborts **all** future rebuilds with `UnsafeWipe`,
telling the user to delete their own edit (`cli/src/main.rs:751-761`).

### 4.8 `migrate/` is the most destructive code in the repo

- `remove_dir_all` of the user's originals (`main.rs:227`) **before** the
  link-conversion pass runs
- no backup flag
- a panicking fallback `Regex::new(r"(?!)").unwrap()` (`:295`) — `(?!)` is
  unsupported look-around, and this is what hard-fails clippy
- it writes outbound `:linksTo` into the source (`:308-329`) while core reads inbound
  `:hasBacklink` from the target — so a migrated vault reports **zero** backlinks
  until `cerbo index`, which then deletes the migrator's output

### 4.9 Spec drift

- Two live specs contradict each other on what `backrefs.ttl` holds:
  `uuid-object-storage/spec.md:88-98` vs `backlinks/spec.md:51-58`.
- The link wire format is spelled two ways: `cerbo://<uuid>` vs
  `cerbo://objects/<uuid>`.
- Five spec files describe deleted machinery: `rename-cascade`,
  `smart-rename-cascade`, `slug-resolution`, `dynamic-slug-generation`,
  `virtual-path-input`.
- `uuid-object-storage/spec.md:14` contains a `TBD` inside a ratified spec.

### 4.10 Smaller confirmed items

- **Windows:** `vault_symlink.rs:352` uses `symlink_dir`, but the target has been
  `page.md` — a file — since the 2026-05-19 change. Should be `symlink_file`.
- **`cleanup_stale_siblings`** (`vault_symlink.rs:156-168`) matches staging dirs by
  prefix only, with no pid or age filter and no lock, so it can delete another live
  process's staging dir and leave the vault with no `cerbo/` tree.
- **`cerbo index` exits 0 on problems**, violating
  `page-metadata-index/spec.md:140-157`.
- **`cerbo index --page` is not incremental:** three unconditional whole-vault passes
  run after it (`cli/src/main.rs:635-655`), each independently re-reading every
  `meta.ttl` (`metadata_index.rs:157,:184,:211`, plus a fourth scan at `:32-40`).
  3.44 s at 100k pages for 0.5 ms of real work.
- **`backrefs_add` rewrites the target's whole file per incoming link** — 3.70 GB
  written into one 740 KB file for a hub with 9,999 backlinks (`links.rs:52-73`).
- **`object_delete` is `remove_dir_all`** (`object.rs:311-332`) with no trash.

---

## 5. Refuted — not problems

Recorded so they are not re-raised:

- `object_write` does **not** commit content before aborting on metadata failure in
  the way alleged.
- A normal editor save through a symlink lands correctly in `page.md`. Because Cerbo
  writes in place, an in-place editor's bytes go to the right file.
- **Slug immutability across renames is intended, not a bug.** `page-crud/spec.md:105`
  and `uuid-object-storage/spec.md:45` specify title and slug as independent. It is
  undocumented and there is no `cerbo page reslug`, but that is a missing capability.

---

## 6. Assessment

Every confirmed critical is a **write-protocol** defect, not a layout defect. None of
§3.1–§3.4 require changing where bytes live; all four are fixable at the current
layout with no vault migration.

That matters because it separates two decisions that look like one:

1. **Harden the write path.** Uncontroversial, no migration, needed regardless of
   what happens to the layout. This is where the criticals are.
2. **Reconsider the layout.** UUID-in-the-directory-name is what *forced* 573 lines of
   symlink projection, and no amount of hardening makes that projection cheap,
   Windows-safe, two-way, or greppable. But this is a deliberate format change and
   deserves its own proposal.

Do (1) now. Decide (2) separately, and only after answering the open questions below
— particularly the first, which determines whether `meta.ttl` should be repaired or
replaced.

---

## 7. Open questions

These are genuine forks where intent, not analysis, decides the answer.

1. **Is the RDF/SPARQL layer a real product commitment or an aspiration?** The
   Oxigraph triple store is named as the third motivation for the whole UUID model
   (`proposal.md:3`, `design.md:45-48`). But no `.ttl` the code writes is parseable,
   `rio_turtle` has zero use sites, `annotations_read` returns an empty vec, and no
   graph query exists. If it is real, the effort belongs in making `rio_turtle`
   genuine. If it is vestigial, moving metadata to `meta.json` with serde makes the
   escaping and injection defects (§3.2, §4.4, §4.5) *structurally impossible* rather
   than test-guarded, and `cerbo export --turtle` from structured data yields more
   real interop than repairing five bespoke writers.
2. **Is greppable, human-named, externally-editable plain text a core value?** This is
   the one thing hardening cannot deliver at any effort level (§4.6, §4.7). If it is a
   core value, plan the identity-in-frontmatter move — which makes all 573 lines of
   `vault_symlink.rs` deletable — as its own change.
3. **What is the answer on content history?** Atomic rename prevents truncation; it
   does not protect against a bad overwrite, a buggy save, or a rogue watcher. There
   is no backup, snapshot, revision, or git backing anywhere in the workspace. For a
   tool holding the only copy of a user's notes, this deserves an explicit accept or
   reject.
4. **Should `cerbo symlink` stay explicit?** Auto-materializing after every
   create/delete/rename is tempting, but `materialize` re-reads every `meta.ttl` and
   recreates every leaf (`vault_symlink.rs:170-215`) — at 100k pages one new page
   costs ~100k `meta.ttl` reads plus ~100k `symlink()` calls. Keep it explicit until
   the projection is incremental.
