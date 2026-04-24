---
name: cluster-8
description: "Skill for the Cluster_8 area of cerbo. 4 symbols across 1 files."
---

# Cluster_8

4 symbols | 1 files | Cohesion: 55%

## When to Use

- Working with code in `core/`
- Understanding how page_list, extract_title work
- Modifying cluster_8-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `core/src/page.rs` | page_list, extract_title, create_page_manually, list_pages |

## Entry Points

Start here when exploring this area:

- **`page_list`** (Function) — `core/src/page.rs:230`
- **`extract_title`** (Function) — `core/src/page.rs:251`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `page_list` | Function | `core/src/page.rs` | 230 |
| `extract_title` | Function | `core/src/page.rs` | 251 |
| `create_page_manually` | Function | `core/src/page.rs` | 275 |
| `list_pages` | Function | `core/src/page.rs` | 300 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Page_list → Chrono_now` | cross_community | 6 |
| `Page_list → Config_dir` | cross_community | 5 |
| `List_pages → Chrono_now` | cross_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_2 | 2 calls |
| Cluster_1 | 1 calls |
| Cluster_9 | 1 calls |

## How to Explore

1. `gitnexus_context({name: "page_list"})` — see callers and callees
2. `gitnexus_query({query: "cluster_8"})` — find related execution flows
3. Read key files listed above for implementation details
