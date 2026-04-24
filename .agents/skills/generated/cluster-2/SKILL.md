---
name: cluster-2
description: "Skill for the Cluster_2 area of cerbo. 18 symbols across 7 files."
---

# Cluster_2

18 symbols | 7 files | Cohesion: 72%

## When to Use

- Working with code in `core/`
- Understanding how run, start_watcher, get_vault_path work
- Modifying cluster_2-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `core/src/index.rs` | new, chrono_now, extract_title, build_index, save_index (+5) |
| `src-tauri/src/lib.rs` | run, vault_open |
| `cli/src/main.rs` | get_context, main |
| `src-tauri/src/main.rs` | main |
| `src-tauri/src/index.rs` | start_watcher |
| `core/src/vault.rs` | get_vault_path |
| `core/src/page.rs` | sync_markdown_titles |

## Entry Points

Start here when exploring this area:

- **`run`** (Function) — `src-tauri/src/lib.rs:21`
- **`start_watcher`** (Function) — `src-tauri/src/index.rs:20`
- **`get_vault_path`** (Function) — `core/src/vault.rs:124`
- **`sync_markdown_titles`** (Function) — `core/src/page.rs:115`
- **`extract_title`** (Function) — `core/src/index.rs:68`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `run` | Function | `src-tauri/src/lib.rs` | 21 |
| `start_watcher` | Function | `src-tauri/src/index.rs` | 20 |
| `get_vault_path` | Function | `core/src/vault.rs` | 124 |
| `sync_markdown_titles` | Function | `core/src/page.rs` | 115 |
| `extract_title` | Function | `core/src/index.rs` | 68 |
| `build_index` | Function | `core/src/index.rs` | 82 |
| `save_index` | Function | `core/src/index.rs` | 117 |
| `create_watcher` | Function | `core/src/index.rs` | 171 |
| `main` | Function | `src-tauri/src/main.rs` | 16 |
| `vault_open` | Function | `src-tauri/src/lib.rs` | 65 |
| `new` | Function | `core/src/index.rs` | 33 |
| `chrono_now` | Function | `core/src/index.rs` | 42 |
| `default` | Function | `core/src/index.rs` | 166 |
| `test_extract_title_from_heading` | Function | `core/src/index.rs` | 205 |
| `test_extract_title_fallback` | Function | `core/src/index.rs` | 211 |
| `test_build_index` | Function | `core/src/index.rs` | 254 |
| `get_context` | Function | `cli/src/main.rs` | 86 |
| `main` | Function | `cli/src/main.rs` | 97 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Attachment_ops → Chrono_now` | cross_community | 7 |
| `Attachment_open → Chrono_now` | cross_community | 7 |
| `Attachment_delete → Chrono_now` | cross_community | 7 |
| `Main → Chrono_now` | cross_community | 6 |
| `Page_rename → Chrono_now` | cross_community | 6 |
| `Page_write → Chrono_now` | cross_community | 6 |
| `Page_list → Chrono_now` | cross_community | 6 |
| `Vault_open → Chrono_now` | cross_community | 6 |
| `Start_watcher → Chrono_now` | cross_community | 6 |
| `Page_read → Chrono_now` | cross_community | 6 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_3 | 3 calls |
| Cluster_4 | 3 calls |
| Cluster_0 | 2 calls |
| Cluster_7 | 1 calls |
| Cluster_10 | 1 calls |

## How to Explore

1. `gitnexus_context({name: "run"})` — see callers and callees
2. `gitnexus_query({query: "cluster_2"})` — find related execution flows
3. Read key files listed above for implementation details
