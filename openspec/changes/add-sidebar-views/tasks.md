# Tasks (DRAFT, after open questions are settled)

## 1. Core
- [x] 1.1 Add `ObjectType::View` + class IRI; exclude from `page_list`
- [x] 1.2 `core/src/view.rs`: `view.toml` read/write (atomic), validation
- [x] 1.3 `view_list/get/create/update/delete`; delete goes to trash
- [x] 1.4 `schema:hasPart` triples in `meta.ttl` for referenced pages
- [x] 1.5 Query node evaluation through the SPARQL engine, row cap

## 2. Transport
- [x] 2.1 Tauri commands + registration in `src-tauri/src/lib.rs`
- [x] 2.2 Daemon RPC calls in `core/src/rpc`
- [x] 2.3 CLI `cerbo view ...`, man page (`cli/man/cerbo.md`), `docs/cli.md`
- [x] 2.4 MCP tools, permission gating, skill text update

## 3. Frontend
- [x] 3.1 `buildViewTree` in `page-tree.ts`, shared flatten/keyboard
- [x] 3.2 `ViewSwitcher.svelte`, `activeViewId` in store and `ui.toml`
- [ ] 3.3 View editing (reorder, add/remove reference done; new folder in a view still open)
- [x] 3.4 Missing-page and query-error rows

## 4. Tests and docs
- [x] 4.1 Core unit tests, CLI integration tests
- [ ] 4.2 e2e: switch view, agent-created view appears, default view intact
- [x] 4.3 `docs/desktop.md`, specs under `specs/`
