---
name: cluster-0
description: "Skill for the Cluster_0 area of cerbo. 17 symbols across 4 files."
---

# Cluster_0

17 symbols | 4 files | Cohesion: 89%

## When to Use

- Working with code in `src-tauri/`
- Understanding how vault_add, vault_remove, vault_list work
- Modifying cluster_0-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src-tauri/src/page.rs` | page_create, page_read, page_write, page_delete, page_list (+4) |
| `src-tauri/src/vault.rs` | vault_add, vault_remove, vault_list, vault_set_active, vault_update_last_page (+1) |
| `src-tauri/src/rename.rs` | page_rename |
| `src-tauri/src/lib.rs` | get_context |

## Entry Points

Start here when exploring this area:

- **`vault_add`** (Function) — `src-tauri/src/vault.rs:5`
- **`vault_remove`** (Function) — `src-tauri/src/vault.rs:10`
- **`vault_list`** (Function) — `src-tauri/src/vault.rs:15`
- **`vault_set_active`** (Function) — `src-tauri/src/vault.rs:20`
- **`vault_update_last_page`** (Function) — `src-tauri/src/vault.rs:26`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `vault_add` | Function | `src-tauri/src/vault.rs` | 5 |
| `vault_remove` | Function | `src-tauri/src/vault.rs` | 10 |
| `vault_list` | Function | `src-tauri/src/vault.rs` | 15 |
| `vault_set_active` | Function | `src-tauri/src/vault.rs` | 20 |
| `vault_update_last_page` | Function | `src-tauri/src/vault.rs` | 26 |
| `vault_relocate` | Function | `src-tauri/src/vault.rs` | 32 |
| `page_rename` | Function | `src-tauri/src/rename.rs` | 6 |
| `page_create` | Function | `src-tauri/src/page.rs` | 6 |
| `page_read` | Function | `src-tauri/src/page.rs` | 12 |
| `page_write` | Function | `src-tauri/src/page.rs` | 18 |
| `page_delete` | Function | `src-tauri/src/page.rs` | 29 |
| `page_list` | Function | `src-tauri/src/page.rs` | 35 |
| `attachment_list` | Function | `src-tauri/src/page.rs` | 41 |
| `attachment_add` | Function | `src-tauri/src/page.rs` | 51 |
| `attachment_upload` | Function | `src-tauri/src/page.rs` | 62 |
| `attachment_delete` | Function | `src-tauri/src/page.rs` | 74 |
| `get_context` | Function | `src-tauri/src/lib.rs` | 11 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Vault_open → CerboContext` | cross_community | 3 |
| `Backlinks_get → CerboContext` | cross_community | 3 |
| `Start_watcher → CerboContext` | cross_community | 3 |
| `Attachment_open → CerboContext` | cross_community | 3 |
| `Vault_add → CerboContext` | intra_community | 3 |
| `Vault_remove → CerboContext` | intra_community | 3 |
| `Vault_list → CerboContext` | intra_community | 3 |
| `Vault_set_active → CerboContext` | intra_community | 3 |
| `Vault_update_last_page → CerboContext` | intra_community | 3 |
| `Vault_relocate → CerboContext` | intra_community | 3 |

## How to Explore

1. `gitnexus_context({name: "vault_add"})` — see callers and callees
2. `gitnexus_query({query: "cluster_0"})` — find related execution flows
3. Read key files listed above for implementation details
