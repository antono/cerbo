---
name: cluster-4
description: "Skill for the Cluster_4 area of cerbo. 12 symbols across 3 files."
---

# Cluster_4

12 symbols | 3 files | Cohesion: 84%

## When to Use

- Working with code in `core/`
- Understanding how load_vaults, save_vaults, vault_add work
- Modifying cluster_4-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `core/src/vault.rs` | vaults_path, load_vaults, save_vaults, vault_add, vault_remove (+4) |
| `core/src/paths.rs` | config_dir, cache_dir |
| `core/src/index.rs` | index_path |

## Entry Points

Start here when exploring this area:

- **`load_vaults`** (Function) — `core/src/vault.rs:27`
- **`save_vaults`** (Function) — `core/src/vault.rs:36`
- **`vault_add`** (Function) — `core/src/vault.rs:48`
- **`vault_remove`** (Function) — `core/src/vault.rs:68`
- **`vault_list`** (Function) — `core/src/vault.rs:86`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `load_vaults` | Function | `core/src/vault.rs` | 27 |
| `save_vaults` | Function | `core/src/vault.rs` | 36 |
| `vault_add` | Function | `core/src/vault.rs` | 48 |
| `vault_remove` | Function | `core/src/vault.rs` | 68 |
| `vault_list` | Function | `core/src/vault.rs` | 86 |
| `vault_set_active` | Function | `core/src/vault.rs` | 90 |
| `vault_update_last_page` | Function | `core/src/vault.rs` | 99 |
| `vault_relocate` | Function | `core/src/vault.rs` | 107 |
| `config_dir` | Function | `core/src/paths.rs` | 3 |
| `cache_dir` | Function | `core/src/paths.rs` | 9 |
| `vaults_path` | Function | `core/src/vault.rs` | 23 |
| `index_path` | Function | `core/src/index.rs` | 125 |

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
| `Attachment_ops → Config_dir` | cross_community | 6 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_2 | 1 calls |

## How to Explore

1. `gitnexus_context({name: "load_vaults"})` — see callers and callees
2. `gitnexus_query({query: "cluster_4"})` — find related execution flows
3. Read key files listed above for implementation details
