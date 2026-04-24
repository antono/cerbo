---
name: cluster-7
description: "Skill for the Cluster_7 area of cerbo. 6 symbols across 1 files."
---

# Cluster_7

6 symbols | 1 files | Cohesion: 71%

## When to Use

- Working with code in `core/`
- Understanding how page_path, page_read, page_write work
- Modifying cluster_7-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `core/src/page.rs` | page_path, page_read, page_write, has_h1, humanize_slug (+1) |

## Entry Points

Start here when exploring this area:

- **`page_path`** (Function) — `core/src/page.rs:23`
- **`page_read`** (Function) — `core/src/page.rs:46`
- **`page_write`** (Function) — `core/src/page.rs:52`
- **`has_h1`** (Function) — `core/src/page.rs:78`
- **`humanize_slug`** (Function) — `core/src/page.rs:82`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `page_path` | Function | `core/src/page.rs` | 23 |
| `page_read` | Function | `core/src/page.rs` | 46 |
| `page_write` | Function | `core/src/page.rs` | 52 |
| `has_h1` | Function | `core/src/page.rs` | 78 |
| `humanize_slug` | Function | `core/src/page.rs` | 82 |
| `ensure_page_has_h1` | Function | `core/src/page.rs` | 99 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Page_write → Chrono_now` | cross_community | 6 |
| `Page_read → Chrono_now` | cross_community | 6 |
| `Page_write → Config_dir` | cross_community | 5 |
| `Page_read → Config_dir` | cross_community | 5 |
| `Start_watcher → Has_h1` | cross_community | 4 |
| `Start_watcher → Humanize_slug` | cross_community | 4 |
| `Test_sync_markdown_titles → Has_h1` | cross_community | 4 |
| `Test_sync_markdown_titles → Humanize_slug` | cross_community | 4 |
| `Test_page_write_infers_title → Has_h1` | cross_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_1 | 2 calls |
| Cluster_8 | 1 calls |

## How to Explore

1. `gitnexus_context({name: "page_path"})` — see callers and callees
2. `gitnexus_query({query: "cluster_7"})` — find related execution flows
3. Read key files listed above for implementation details
