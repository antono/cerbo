---
name: cluster-3
description: "Skill for the Cluster_3 area of cerbo. 12 symbols across 5 files."
---

# Cluster_3

12 symbols | 5 files | Cohesion: 68%

## When to Use

- Working with code in `core/`
- Understanding how backlinks_get, page_rename, load_index work
- Modifying cluster_3-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `core/src/rename.rs` | page_rename, get_all_slugs, extract_title_from_content, rewrite_heading, rewrite_heading_replaces_first_h1 (+1) |
| `core/src/index.rs` | load_index, compute_backlinks, test_compute_backlinks |
| `src-tauri/src/index.rs` | backlinks_get |
| `core/src/fixtures.rs` | create_fixture_vault |
| `cli/src/main.rs` | test_cli_rename_cascade |

## Entry Points

Start here when exploring this area:

- **`backlinks_get`** (Function) — `src-tauri/src/index.rs:9`
- **`page_rename`** (Function) — `core/src/rename.rs:12`
- **`load_index`** (Function) — `core/src/index.rs:111`
- **`compute_backlinks`** (Function) — `core/src/index.rs:133`
- **`create_fixture_vault`** (Function) — `core/src/fixtures.rs:17`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `backlinks_get` | Function | `src-tauri/src/index.rs` | 9 |
| `page_rename` | Function | `core/src/rename.rs` | 12 |
| `load_index` | Function | `core/src/index.rs` | 111 |
| `compute_backlinks` | Function | `core/src/index.rs` | 133 |
| `create_fixture_vault` | Function | `core/src/fixtures.rs` | 17 |
| `get_all_slugs` | Function | `core/src/rename.rs` | 83 |
| `extract_title_from_content` | Function | `core/src/rename.rs` | 101 |
| `rewrite_heading` | Function | `core/src/rename.rs` | 111 |
| `rewrite_heading_replaces_first_h1` | Function | `core/src/rename.rs` | 230 |
| `test_rename_cascade_smart` | Function | `core/src/rename.rs` | 238 |
| `test_compute_backlinks` | Function | `core/src/index.rs` | 217 |
| `test_cli_rename_cascade` | Function | `cli/src/main.rs` | 227 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Page_rename → Chrono_now` | cross_community | 6 |
| `Test_rename_cascade_smart → Config_dir` | cross_community | 6 |
| `Test_cli_rename_cascade → Chrono_now` | cross_community | 5 |
| `Test_cli_rename_cascade → Cache_dir` | cross_community | 5 |
| `Test_rename_cascade_smart → Chrono_now` | cross_community | 5 |
| `Test_rename_cascade_smart → Cache_dir` | cross_community | 5 |
| `Vault_open → Cache_dir` | cross_community | 4 |
| `Backlinks_get → Cache_dir` | cross_community | 4 |
| `Test_cli_rename_cascade → Extract_title` | cross_community | 4 |
| `Test_cli_rename_cascade → Extract_wikilinks` | cross_community | 4 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_2 | 6 calls |
| Cluster_0 | 1 calls |
| Cluster_1 | 1 calls |
| Cluster_6 | 1 calls |
| Cluster_4 | 1 calls |

## How to Explore

1. `gitnexus_context({name: "backlinks_get"})` — see callers and callees
2. `gitnexus_query({query: "cluster_3"})` — find related execution flows
3. Read key files listed above for implementation details
