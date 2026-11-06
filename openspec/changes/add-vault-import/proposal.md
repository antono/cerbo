# Proposal

## Why

Most people who try Cerbo already keep notes in an Obsidian or Tolaria vault:
a folder of Markdown files with YAML frontmatter and `[[wikilinks]]`. Today
the only way in is `cerbo page create` + `cerbo page write` per note, with
every link rewritten by hand. A vault of 137 notes is a day of work; with an
importer it is one command, and the frontmatter (types, relationships) lands
in the knowledge graph instead of being lost.

## What Changes

- **New command `cerbo import-vault <DIR>`** imports an Obsidian or Tolaria
  vault into a Cerbo vault. The source folder is only read, never changed.
- **Format detection:** `.obsidian/` means Obsidian; Tolaria markers (a
  `views/*.yml` folder, notes with a `type:` frontmatter field and no
  `.obsidian/`) mean Tolaria. `--from obsidian|tolaria` overrides; an
  undetectable folder without `--from` is refused.
- **Target vault is chosen by the user:** with no flag, an interactive prompt
  lists the registered vaults plus "Create a new vault". `--vault <ID>` picks
  a registered vault, `--new <PATH> [--name <NAME>]` runs `cerbo init` there
  and registers it. Without a TTY and without either flag the command fails
  and writes nothing.
- **Notes become `:Product` pages:** title from the first H1 (Tolaria) or the
  file name (Obsidian), folder becomes `cerbo:virtualPath`, file stem becomes
  `cerbo:slug`, file times become `schema:dateCreated`/`schema:dateModified`.
- **Links are rewritten:** `[[Note]]`, `[[Note|text]]`, `[[Note#Heading]]`
  become `[text](cerbo://<uuid>)`; embeds and Markdown links to local files
  become Attachment objects linked as `![alt](cerbo://<uuid>)`. Unresolved
  links stay as text and are reported.
- **Frontmatter becomes RDF in `meta.ttl`:** `type` becomes an extra
  `rdf:type`, properties holding wikilinks become object relations, other
  values become literals, all in a namespace owned by one Ontology object per
  import. Tolaria's `_`-prefixed app state is dropped. `page.md` holds no
  frontmatter.
- **One-shot import:** the import is recorded; importing the same source
  folder into the same vault again is refused unless `--force`, which creates
  a second, separate copy. Updating an earlier import is out of scope.
- `--dry-run` prints the plan (counts, unresolved links, skipped files)
  without writing; `--json` gives the report as JSON.

## Capabilities

### New Capabilities

- `vault-import`: importing a foreign Markdown vault (Obsidian, Tolaria):
  detection, target selection, note/attachment mapping, link rewriting,
  frontmatter to RDF, import record, report.

### Modified Capabilities

- `cli-tooling`: new `cerbo import-vault` command and man page section.

## Non-goals

- Syncing or re-importing changes from the source vault (v1 is one-shot).
- Exporting a Cerbo vault back to Obsidian/Tolaria.
- Obsidian plugins' data (Dataview queries, Canvas `.canvas`, Excalidraw),
  Tolaria saved views (`views/*.yml`). They are listed as skipped.
- Logseq, Notion, Bear and other formats; the design leaves room for them.
- Transclusion: `![[Note]]` becomes an ordinary link, not embedded content.
- Desktop UI for import. The CLI comes first.

## Impact

- **Core:** new module `core/src/vault_import/` (scan, detect, plan, write),
  reusing `object_create_with_metadata`, `ObjectMeta::to_turtle_with`,
  `prefix_map`, `slug::slugify`. A YAML parser dependency (`serde_yaml_ng` or
  similar) is added to `core`.
- **CLI:** `cli/src/main.rs`, `cli/man/cerbo.md`, `cli/tests/`.
- **Nix:** `Cargo.lock` change only; no Bun dependency changes.
