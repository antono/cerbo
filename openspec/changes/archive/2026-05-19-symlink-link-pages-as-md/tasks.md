## 1. Core Logic (`core/src/vault_symlink.rs`)

- [x] 1.1 Change object type filter from skip-Ontology to allow-only-Product-or-Source
- [x] 1.2 Add `page.md` existence check; skip with stderr warning if absent
- [x] 1.3 Change symlink target from object directory to `<uuid>/page.md`
- [x] 1.4 Append `.md` extension to leaf name in path construction (`format!("{}.md", slug)`)
- [x] 1.5 Update `rendered_path()` to include `.md` extension so collision detection uses canonical paths

## 2. Tests (`core/src/vault_symlink.rs`)

- [x] 2.1 Update existing scenario tests: leaf names gain `.md`, target paths end with `/page.md`
- [x] 2.2 Add test: Attachment object produces no symlink
- [x] 2.3 Add test: Product object missing `page.md` is skipped and warning emitted to stderr
- [x] 2.4 Add test: symlink target resolves to `page.md`, not the object directory

## 3. Man Page (`cli/man/cerbo.md`)

- [x] 3.1 Update example output and description to show `<slug>.md` leaves and `page.md` targets
