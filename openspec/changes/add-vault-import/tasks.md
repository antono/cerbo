# Tasks

## 1. Fixtures and dependencies

- [ ] 1.1 Add test fixtures under `core/tests/fixtures/import/`: a small Obsidian vault (`.obsidian/app.json`, nested folders, `![[photo.png]]`, `[[Note|alias]]`, `[[Note#Heading]]`, `aliases`/`tags` frontmatter, a `.canvas` file, a duplicate stem in two folders, a wikilink inside a code fence) and a small Tolaria vault modelled on `~/Sync/Tolaria/Braindump` (`type:` notes, a `type: Type` note, `_favorite` keys, `belongs_to`/`related_to` relationships, `attachments/pdf/` links, `views/x.yml`, a Cyrillic file name, broken frontmatter); verify the fixtures contain no personal data
- [ ] 1.2 Add `serde-saphyr` and `pulldown-cmark` to `core/Cargo.toml` (check current versions first), regenerate `Cargo.lock`; verify `nix build .#cerbo` succeeds

## 2. Core: scan and plan

- [ ] 2.1 Create `core/src/vault_import/` with `scan` (walk source, skip rules, file kinds, birth/modified times) and `detect` (Obsidian/Tolaria/unknown, refuse `.cerbo/`); verify unit tests on both fixtures, an empty folder and a Cerbo vault
- [ ] 2.2 Implement frontmatter split and parse into the `FrontValue` enum with `serde-saphyr`, keeping the raw block on parse failure; verify tests for scalars, lists, nested maps, dates, quoted wikilinks, broken YAML
- [ ] 2.3 Implement the `Dialect` trait with `Obsidian` and `Tolaria` (title, dropped keys, special keys, skip lists); verify title tests (H1 vs stem, missing H1) and that `_favorite` is dropped
- [ ] 2.4 Build `ImportPlan`: UUIDs for notes and referenced files, slugs with `-2` suffixing per folder, virtual paths, resolution index (path, stem, title, alias) with nearest-folder tie-break; verify tests for every lookup order step, ambiguity and slug collisions
- [ ] 2.5 Implement body rewriting outside code ranges: wikilinks, embeds, relative Markdown links to notes and files, H1 prepend, unresolved kept verbatim; verify tests for each spec scenario under "Wikilink rewriting", "Markdown links to notes" and "Attachments", plus code fence and inline code untouched
- [ ] 2.6 Map frontmatter to triples in `cerbo://vocab/<uuid>/` (type, relationships, literals with datatypes, tags, aliases, percent-encoded keys); verify tests for each frontmatter scenario and that a relationship to an unresolved note becomes a literal
- [ ] 2.7 Produce `ImportReport` (serde, camelCase) from the plan; verify the JSON field names match the spec

## 3. Core: write

- [ ] 3.1 Implement staging in `.cerbo/import-staging/<id>/` with `manifest.json`, writing pages (`meta.ttl` via `to_turtle_with`, `page.md`, `relations.ttl`), attachments and the ontology object through the existing atomic write helpers; verify a staged import of the Tolaria fixture produces valid Turtle that `ObjectMeta::read_from_file` loads
- [ ] 3.2 Implement commit (rename attachments, pages, ontology; then prefix map with unique prefix) and leftover-staging cleanup per design D1; verify a test that injects a failure after N renames and checks the next run trashes the committed objects and leaves `page list` as before
- [ ] 3.3 Implement the repeat-import check (ontology objects with the same canonical `schema:url` and `cerbo:importFormat`) and `--force`; verify tests for refuse and forced second copy
- [ ] 3.4 Run one incremental semantic index refresh after commit; verify a test that `backlinks` of an imported page lists its importer page without `cerbo index`
- [ ] 3.5 Verify the source is untouched: a test hashes every fixture file and its mtime before and after a full import

## 4. CLI

- [ ] 4.1 Add `cerbo import-vault <DIR> [--from] [--vault | --new [--name]] [--force] [--dry-run] [--json]` in `cli/src/main.rs` with a clap group making `--vault`/`--new` exclusive; verify integration tests for usage errors and unknown vault id
- [ ] 4.2 Implement the interactive prompt (stderr list, stdin answer, `IsTerminal` gate, "Create a new vault" asks path and name, vault created only after planning succeeds); verify an integration test feeding stdin through a pty and one asserting the non-TTY error
- [ ] 4.3 Print the human report and the `--json` report; verify integration tests on both fixtures (counts, unresolved list, `--dry-run` writes nothing and registry unchanged)
- [ ] 4.4 Document `import-vault` in `cli/man/cerbo.md` (options, detection, mapping table, Obsidian and Tolaria examples) and add it to `.agents/skills/cerbo-cli/`; verify `nix build .#cerbo` and the generated man page shows the section

## 5. Verification

- [ ] 5.1 Dry-run against a copy of a real Tolaria vault (`~/Sync/Tolaria/Braindump`) and review the report for unexpected unresolved links; record findings in this change's design if rules need adjusting
- [ ] 5.2 Run `cargo test` and `cargo clippy` inside `nix develop`, and `nix build .#cerbo .#cerbo-desktop`; verify all pass
- [ ] 5.3 Run `openspec validate add-vault-import --strict`; verify it passes

## Workflow follow-up

- Archive the change with `/opsx-archive` after merge.
