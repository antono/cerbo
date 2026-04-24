---
name: cluster-9
description: "Skill for the Cluster_9 area of cerbo. 4 symbols across 1 files."
---

# Cluster_9

4 symbols | 1 files | Cohesion: 67%

## When to Use

- Working with code in `core/`
- Understanding how setup_vault, create_read_delete, test_page_write_infers_title work
- Modifying cluster_9-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `core/src/page.rs` | setup_vault, create_read_delete, test_page_write_infers_title, test_sync_markdown_titles |

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `setup_vault` | Function | `core/src/page.rs` | 269 |
| `create_read_delete` | Function | `core/src/page.rs` | 282 |
| `test_page_write_infers_title` | Function | `core/src/page.rs` | 328 |
| `test_sync_markdown_titles` | Function | `core/src/page.rs` | 350 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Test_sync_markdown_titles → Chrono_now` | cross_community | 4 |
| `Test_sync_markdown_titles → Has_h1` | cross_community | 4 |
| `Test_sync_markdown_titles → Humanize_slug` | cross_community | 4 |
| `Test_page_write_infers_title → Has_h1` | cross_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_7 | 1 calls |
| Cluster_2 | 1 calls |

## How to Explore

1. `gitnexus_context({name: "setup_vault"})` — see callers and callees
2. `gitnexus_query({query: "cluster_9"})` — find related execution flows
3. Read key files listed above for implementation details
