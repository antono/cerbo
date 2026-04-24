---
name: cluster-10
description: "Skill for the Cluster_10 area of cerbo. 4 symbols across 1 files."
---

# Cluster_10

4 symbols | 1 files | Cohesion: 86%

## When to Use

- Working with code in `core/`
- Understanding how extract_wikilinks work
- Modifying cluster_10-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `core/src/index.rs` | extract_wikilinks, test_extract_wikilinks_basic, test_extract_wikilinks_empty, test_extract_wikilinks_unclosed |

## Entry Points

Start here when exploring this area:

- **`extract_wikilinks`** (Function) — `core/src/index.rs:50`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `extract_wikilinks` | Function | `core/src/index.rs` | 50 |
| `test_extract_wikilinks_basic` | Function | `core/src/index.rs` | 187 |
| `test_extract_wikilinks_empty` | Function | `core/src/index.rs` | 193 |
| `test_extract_wikilinks_unclosed` | Function | `core/src/index.rs` | 199 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Test_cli_rename_cascade → Extract_wikilinks` | cross_community | 4 |
| `Test_rename_cascade_smart → Extract_wikilinks` | cross_community | 4 |
| `Main → Extract_wikilinks` | cross_community | 3 |
| `Vault_open → Extract_wikilinks` | cross_community | 3 |
| `Start_watcher → Extract_wikilinks` | cross_community | 3 |

## How to Explore

1. `gitnexus_context({name: "extract_wikilinks"})` — see callers and callees
2. `gitnexus_query({query: "cluster_10"})` — find related execution flows
3. Read key files listed above for implementation details
