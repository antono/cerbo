## 1. Foundation: dependencies and shared helpers

- [ ] 1.1 Add `slug = "0.1"` and `pathdiff = "0.2"` to `core/Cargo.toml`; regenerate `nix/bun.nix` is NOT needed (Cargo-only deps), but run `cargo build -p cerbo-core` inside `nix develop` to update the lockfile and confirm both crates compile on the project's `edition = "2024"`
- [ ] 1.2 Add a `core::slug` module exposing `slugify(title: &str, uuid: Uuid) -> String` implementing the project slug algorithm: deunicode transliteration via the `slug` crate, force lowercase, cap at 80 chars (truncating on a `-` boundary when possible), trim trailing `-`, fall back to `untitled-<first-8-of-uuid>` when the result is empty. Include unit tests covering: plain English, Cyrillic, German umlauts, emoji-only, length > 80, length-cap-on-boundary, whitespace-only
- [ ] 1.3 Add `core::vault::find_repository_root(start: &Path) -> Option<PathBuf>` that walks upward looking for a directory containing a `.cerbo/` subdirectory, stopping at filesystem mount-point boundaries (detect via `std::fs::metadata().dev()` change) and at the filesystem root. Include unit tests using `tempfile`: repo at start, repo in ancestor, no repo, simulated mount boundary
- [ ] 1.4 Add `core::vault::validate_virtual_path(s: &str) -> Result<(), VirtualPathError>` enforcing: no leading/trailing `/`, no `.` or `..` segments, no empty segments, no NUL bytes. Empty string is valid (root). Unit tests for each rejection case and several valid cases

## 2. `meta.ttl` extensions: `cerbo:slug` and `cerbo:virtualPath`

- [ ] 2.1 Extend the `meta.ttl` parser in `core/src/object.rs` to read `cerbo:slug` and `cerbo:virtualPath` predicates from the existing `cerbo:` namespace (`https://cerbo.app/ns#`). Both predicates are optional and read as `Option<String>`. Reading a page that lacks them MUST still succeed
- [ ] 2.2 Extend the `meta.ttl` serializer to write `cerbo:slug` and `cerbo:virtualPath` triples when present. Round-trip property: parse → serialize → parse yields identical predicate values
- [ ] 2.3 On parse, validate `cerbo:slug` shape (1..=80 chars, kebab-case ASCII, no `/`, no leading/trailing `-`) and `cerbo:virtualPath` shape using `validate_virtual_path` (task 1.4). Invalid values produce a structured error logged by the caller; the parser MUST NOT panic
- [ ] 2.4 Unit tests: page with both predicates, page with only `cerbo:slug`, page with only `cerbo:virtualPath`, page with neither, page with invalid slug, page with invalid virtualPath

## 3. `page-crud`: slug auto-generation on create

- [ ] 3.1 Update `core::page::create_page` (or the equivalent CRUD entry point) so that creating a new page always writes `cerbo:slug` derived from the title via `core::slug::slugify`, unless the caller passes an explicit slug override
- [ ] 3.2 Add an optional `virtual_path: Option<String>` parameter to the page-creation API. When supplied, validate via `validate_virtual_path` and write the predicate. When omitted, do NOT write the predicate
- [ ] 3.3 Update existing `cerbo create` CLI invocation in `cli/src/main.rs` to populate `cerbo:slug` on the created page (no new flag needed for v1; CLI does not yet expose `--virtual-path`)
- [ ] 3.4 Update existing unit/integration tests in `core::page` to assert that newly-created pages have a `cerbo:slug` matching the derived value

## 4. `cerbo index` extensions

- [ ] 4.1 In `core::metadata_index`, add a slug-backfill pass: for every Page or Source object lacking `cerbo:slug`, compute the derived slug and write it to `meta.ttl`. Skip `:Ontology` objects entirely. Emit one stderr line per backfilled object
- [ ] 4.2 Add a `--no-backfill-slug` flag to the `cerbo index` clap subcommand that disables the backfill pass; when set, missing slugs are reported to stderr but `meta.ttl` is not modified
- [ ] 4.3 Add a virtualPath-validation pass that reports invalid `cerbo:virtualPath` values to stderr (UUID + reason) without modifying `meta.ttl`. Invalid values do NOT cause `cerbo index` to abort; they are reported alongside other warnings
- [ ] 4.4 Add a combined-path collision-detection pass that computes `<virtualPath>/<slug>` for every non-ontology object and reports (a) leaf-vs-leaf and (b) dir-vs-leaf collisions to stderr with offending UUIDs. Detected collisions cause `cerbo index` to exit non-zero
- [ ] 4.5 Integration tests: small fixture vault with (a) missing-slug page, (b) invalid-virtualPath page, (c) leaf-vs-leaf colliding pair, (d) dir-vs-leaf colliding pair. Assert stderr content and exit code for each

## 5. `cerbo init`: `.gitignore` handling

- [ ] 5.1 In the `cerbo init` implementation, after creating `.cerbo/`, ensure a `.gitignore` exists at the repository root: if absent, create with `/cerbo/\n`; if present and contains a line exactly matching `/cerbo/`, no-op; if present and missing the line, append a section with a one-line comment `# Cerbo symlink tree (regenerate with: cerbo symlink)` and the `/cerbo/` line
- [ ] 5.2 Integration tests: init in empty dir (creates .gitignore), init in dir with pre-existing matching .gitignore (no-op), init in dir with .gitignore missing the line (appends), re-init on existing cerbo repo (still idempotent for both `.cerbo/` and `.gitignore`)

## 6. `vault-symlink`: core algorithm in `core::vault::symlink`

- [ ] 6.1 New module `core/src/vault_symlink.rs` (or `core::vault::symlink`) exposing a public `materialize(repo_root: &Path) -> Result<MaterializeReport, SymlinkError>`. `MaterializeReport` carries counts (objects scanned, leaves created, directories created) for the CLI summary
- [ ] 6.2 Plan-building step: scan `<repo-root>/.cerbo/objects/*/meta.ttl`, skipping `:Ontology`. Produce a `Vec<PlanEntry { uuid, virtual_path: String, slug: String }>` using each page's explicit `cerbo:slug` or the derived fallback, and explicit `cerbo:virtualPath` or empty string
- [ ] 6.3 Plan-validation step: build an in-memory representation of the projected tree (e.g. `BTreeMap<PathBuf, Node>` where `Node` is either `Symlink { uuid }` or `Directory`). Detect leaf-vs-leaf and dir-vs-leaf conflicts; on any conflict, return `SymlinkError::Conflict { collisions: Vec<Collision> }` listing each collision with its path and contributing UUIDs. Do NOT touch the filesystem when conflicts are present
- [ ] 6.4 Safe-wipe step: if `<repo-root>/cerbo/` exists, walk it recursively; verify every entry is either a directory (recurse) or a symlink whose `std::fs::canonicalize`-resolved target lives under `<repo-root>/.cerbo/objects/`. Any other entry → `SymlinkError::UnsafeWipe { offenders: Vec<PathBuf> }`. Existing tree must remain untouched on error
- [ ] 6.5 Stale-sibling cleanup step: at the start of a run, remove any leftover `<repo-root>/cerbo.tmp-*` and `<repo-root>/cerbo.gc-*` directories (they are unambiguously cerbo-owned by name pattern)
- [ ] 6.6 Materialization step: create `<repo-root>/cerbo.tmp-<pid>/`; for each plan entry, `mkdir -p` the `<virtualPath>` chain inside it, then create the leaf symlink with target computed by `pathdiff::diff_paths(<repo-root>/.cerbo/objects/<uuid>/, leaf_parent_dir)`. Use `std::os::unix::fs::symlink` on Unix; gate Windows behind `std::os::windows::fs::symlink_dir` with an explicit hint message on permission error
- [ ] 6.7 Two-phase swap step: if `<repo-root>/cerbo/` exists, `rename` it to `<repo-root>/cerbo.gc-<pid>/`; then `rename` `<repo-root>/cerbo.tmp-<pid>/` to `<repo-root>/cerbo/`; finally `remove_dir_all` `<repo-root>/cerbo.gc-<pid>/` if it exists
- [ ] 6.8 Unit tests in `core/src/vault_symlink.rs`: each helper function (plan builder, validator, safe-wipe walker, relative-target computer) tested in isolation with `tempfile`-backed fixtures
- [ ] 6.9 Integration tests: build a fixture vault with several pages spanning multiple virtualPaths, including the empty path; run `materialize`; assert the resulting tree structure (`readlink` values, directory presence) matches expectation; run a second time; assert no `cerbo.tmp-*` or `cerbo.gc-*` siblings remain
- [ ] 6.10 Integration test for portability: `materialize` a fixture repo, move the whole repo dir to a new path via `fs::rename`, assert every leaf symlink still resolves to a valid object directory

## 7. `cerbo symlink` CLI subcommand

- [ ] 7.1 Add a `Symlink` variant to the top-level clap `Command` enum in `cli/src/main.rs`. The variant takes no positional arguments and no flags for v1
- [ ] 7.2 Handler: call `find_repository_root(&std::env::current_dir()?)`; on `None`, print `not a cerbo repository (or any parent up to mount point)` to stderr and exit 1
- [ ] 7.3 On Some(root), call `core::vault_symlink::materialize(&root)`; on success, print a one-line stdout summary using `MaterializeReport` (e.g. `Symlinked N pages, M directories under <root>/cerbo/`)
- [ ] 7.4 On `SymlinkError::Conflict`, print each collision (path + UUIDs) to stderr and exit 1. On `SymlinkError::UnsafeWipe`, print offending paths to stderr and exit 1. On other errors, print error and exit 1
- [ ] 7.5 CLI integration tests using `assert_cmd`: success path, run-outside-repo path, conflict path, unsafe-wipe path. Use `tempfile` to construct fixture repos
- [ ] 7.6 Reject any positional argument: `cerbo symlink anything` exits non-zero with a clap usage error (verify in test)

## 8. Documentation

- [ ] 8.1 Update the man page (`cli/man/cerbo.1` or wherever it lives) to add a `cerbo symlink` SYNOPSIS, DESCRIPTION, EXIT STATUS, and EXAMPLES section. Document repository discovery, `<repo-root>/cerbo/` output, always-relative symlinks, and full-rebuild semantics
- [ ] 8.2 Update the man page entry for `cerbo init` to mention the `.gitignore` behavior
- [ ] 8.3 Update the man page entry for `cerbo index` to document `--no-backfill-slug` and the new validation/collision behaviors
- [ ] 8.4 Update the README CLI section with a brief `cerbo symlink` usage example and the cerbo-repository definition
- [ ] 8.5 Verify all `--help` strings generated by clap match the man page wording

## 9. Cross-platform and quality

- [ ] 9.1 On Windows, surface a clear actionable error when `symlink_dir` fails with `ERROR_PRIVILEGE_NOT_HELD` (typically because Developer Mode is off). The CLI prints a one-line hint pointing at the Microsoft Developer Mode docs and exits non-zero
- [ ] 9.2 Manual smoke test on macOS (HFS+/APFS): create a repo, run `cerbo symlink`, verify the tree exists and `cd`-ing into a slug works
- [ ] 9.3 Manual smoke test of portability: build a repo with several pages, run `cerbo symlink`, `mv` the repo to a new path, `ls -lR cerbo/` and verify every symlink still resolves (`readlink -f`)
- [ ] 9.4 Run `cargo test --workspace` and `nix build .#cerbo` (both inside `nix develop`) and ensure both pass before merging

## 10. Wrap-up

- [ ] 10.1 Verify the change with `openspec validate add-vault-symlink-command` and `openspec status --change add-vault-symlink-command`
- [ ] 10.2 File follow-up bd issues for any deferred work (Open Questions in design.md, e.g. `--include-ontologies`, `--output <dir>`)
- [ ] 10.3 Archive the change via `openspec archive add-vault-symlink-command` once all tasks are complete and the implementation is merged
