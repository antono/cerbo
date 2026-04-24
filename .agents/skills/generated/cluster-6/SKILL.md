---
name: cluster-6
description: "Skill for the Cluster_6 area of cerbo. 6 symbols across 1 files."
---

# Cluster_6

6 symbols | 1 files | Cohesion: 91%

## When to Use

- Working with code in `core/`
- Understanding how rewrite_links_in_pages, replace_wikilink, replace_wikilink_basic work
- Modifying cluster_6-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `core/src/rename.rs` | rewrite_links_in_pages, replace_wikilink, replace_wikilink_basic, replace_wikilink_case_insensitive, replace_wikilink_no_match (+1) |

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `rewrite_links_in_pages` | Function | `core/src/rename.rs` | 135 |
| `replace_wikilink` | Function | `core/src/rename.rs` | 163 |
| `replace_wikilink_basic` | Function | `core/src/rename.rs` | 202 |
| `replace_wikilink_case_insensitive` | Function | `core/src/rename.rs` | 208 |
| `replace_wikilink_no_match` | Function | `core/src/rename.rs` | 214 |
| `replace_wikilink_multiple` | Function | `core/src/rename.rs` | 220 |

## How to Explore

1. `gitnexus_context({name: "rewrite_links_in_pages"})` — see callers and callees
2. `gitnexus_query({query: "cluster_6"})` — find related execution flows
3. Read key files listed above for implementation details
