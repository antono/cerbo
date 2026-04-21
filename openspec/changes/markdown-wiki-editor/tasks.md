## 1. Project Scaffold

- [ ] 1.1 Run `bun create tauri-app cerbo` with SvelteKit template
- [ ] 1.2 Configure `devenv.sh` with Rust stable toolchain, Bun, and Tauri CLI
- [ ] 1.3 Add devenv shell scripts: `dev` (tauri dev), `build` (tauri build), `check` (cargo check)
- [ ] 1.4 Add Rust dependencies: `serde`, `serde_json`, `uuid`, `walkdir`, `notify`
- [ ] 1.5 Add frontend dependencies: `carta-md`, `shadcn-svelte`
- [ ] 1.6 Verify `bun tauri dev` launches the app window

## 2. XDG Paths

- [ ] 2.1 Add helper `config_dir()` using Tauri's `app_config_dir()` → `$XDG_CONFIG_HOME/cerbo/`
- [ ] 2.2 Add helper `cache_dir(vault_id)` using Tauri's `app_cache_dir()` → `$XDG_CACHE_HOME/cerbo/<vault-id>/`
- [ ] 2.3 Ensure directories are created on first use (mkdir -p equivalent)

## 3. Vault Registry

- [ ] 3.1 Define `Vault` and `VaultsFile` structs (id, name, path, activeVaultId)
- [ ] 3.2 Implement `load_vaults()` — read and deserialize `vaults.json` (return empty registry if missing)
- [ ] 3.3 Implement `save_vaults()` — serialize and write `vaults.json` atomically
- [ ] 3.4 Implement `vault_add(name, path)` Tauri command — validate path exists, assign UUID, persist
- [ ] 3.5 Implement `vault_remove(id)` Tauri command — remove from registry, delete cache dir
- [ ] 3.6 Implement `vault_list()` Tauri command — return all registered vaults
- [ ] 3.7 Implement `vault_set_active(id)` Tauri command — update `activeVaultId` in registry
- [ ] 3.8 Implement `vault_relocate(id, new_path)` Tauri command — update path, keep ID and cache
- [ ] 3.9 Write unit tests for vault add, remove, and relocate

## 4. Slug Derivation

- [ ] 4.1 Implement `derive_slug(title: &str) -> String` in Rust (NFKD → strip diacritics → lowercase → kebab → trim)
- [ ] 4.2 Add Rust crate `unicode-normalization` for NFKD support
- [ ] 4.3 Expose `slug_from_title(title)` as Tauri command
- [ ] 4.4 Write unit tests covering ASCII, diacritics, special chars, leading/trailing punctuation

## 5. Page CRUD

- [ ] 5.1 Implement `page_create(vault_id, title)` Tauri command — derive slug, mkdir, write `# Title` to `page.md`
- [ ] 5.2 Implement `page_read(vault_id, slug)` Tauri command — read `<slug>/page.md` raw content
- [ ] 5.3 Implement `page_write(vault_id, slug, content)` Tauri command — write content atomically
- [ ] 5.4 Implement `page_delete(vault_id, slug)` Tauri command — remove entire page folder
- [ ] 5.5 Implement `page_list(vault_id)` Tauri command — scan vault for folders containing `page.md`, return slug + title
- [ ] 5.6 Reject `page_create` if slug already exists (return descriptive error)
- [ ] 5.7 Write unit tests for create, read, write, delete, and list

## 6. Link Index

- [ ] 6.1 Define `LinkIndex` struct (`version`, `builtAt`, `pages: HashMap<slug, PageEntry { title, links }>`)
- [ ] 6.2 Implement `build_index(vault_path)` — scan all `*/page.md`, extract `[[wikilink]]` occurrences
- [ ] 6.3 Implement `load_index(vault_id)` — read cache JSON, return None if missing or stale
- [ ] 6.4 Implement `save_index(vault_id, index)` — write to `$XDG_CACHE_HOME/cerbo/<vault-id>/index.json`
- [ ] 6.5 Implement `compute_backlinks(index, slug)` — invert index to find all pages linking to slug
- [ ] 6.6 Expose `backlinks_get(vault_id, slug)` Tauri command
- [ ] 6.7 Implement FS watcher with `notify` — watch active vault for `page.md` changes, update index incrementally
- [ ] 6.8 Rebuild full index on vault open if cache is missing or stale
- [ ] 6.9 Write unit tests for build, compute backlinks, and incremental update

## 7. Rename Cascade

- [ ] 7.1 Implement `page_rename(vault_id, old_slug, new_title)` Tauri command
- [ ] 7.2 Validate new slug does not conflict with existing page (reject with error if so)
- [ ] 7.3 Rename folder: `old_slug/` → `new_slug/`
- [ ] 7.4 Scan all `*/page.md` in vault, replace `[[OldTitle]]` variants case-insensitively with `[[NewTitle]]`
- [ ] 7.5 Trigger full link index rebuild after cascade completes
- [ ] 7.6 Write unit tests: rename with no links, rename with links, rename conflict

## 8. Asset Serving

- [ ] 8.1 Configure Tauri `asset://` protocol scope — register vault root as allowed path on vault open
- [ ] 8.2 Update `tauri.conf.json` asset protocol permissions
- [ ] 8.3 Verify relative image references (`![](diagram.png)`) render correctly in Carta for a test page

## 9. Carta Wikilink Plugin

- [ ] 9.1 Create Carta plugin: tokenize `[[...]]` as wikilink syntax
- [ ] 9.2 Fetch page list from Tauri on editor mount; update when vault changes
- [ ] 9.3 Apply resolved style to wikilinks whose slug exists in page list
- [ ] 9.4 Apply broken style to wikilinks with no matching slug
- [ ] 9.5 Add click handler on rendered wikilinks — navigate to page or offer create
- [ ] 9.6 Implement `[[` autocomplete dropdown — filter by title/slug as user types
- [ ] 9.7 Insert selected suggestion as full `[[Page Title]]` wikilink, close dropdown

## 10. UI Shell

- [ ] 10.1 Create vault switcher component (sidebar or header dropdown)
- [ ] 10.2 Create page list component for active vault
- [ ] 10.3 Create page editor view — mount Carta with wikilink plugin
- [ ] 10.4 Create backlinks panel component (sidebar)
- [ ] 10.5 Wire new-page flow: title input → slug preview (via Tauri command) → create
- [ ] 10.6 Wire delete-page flow with confirmation dialog
- [ ] 10.7 Wire rename-page flow: new title input → slug conflict check → confirm

## 11. Integration & Polish

- [ ] 11.1 Handle vault open on app launch — load active vault, build/validate index
- [ ] 11.2 Show loading state while index builds for large vaults
- [ ] 11.3 Display error toasts for Tauri command failures
- [ ] 11.4 Persist last-open page per vault (restore on vault switch)
- [ ] 11.5 Run `cargo clippy` and fix all warnings
- [ ] 11.6 Run `bun check` (svelte-check) and fix all TypeScript errors
- [ ] 11.7 Smoke-test full flow: add vault → create pages → link → navigate → rename → verify cascade
