---
name: cluster-1
description: "Skill for the Cluster_1 area of cerbo. 13 symbols across 4 files."
---

# Cluster_1

13 symbols | 4 files | Cohesion: 83%

## When to Use

- Working with code in `core/`
- Understanding how slug_from_title, attachment_open, derive_slug work
- Modifying cluster_1-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `core/src/page.rs` | vault_root, page_create, page_delete, attachment_list, attachment_add (+4) |
| `core/src/slug.rs` | derive_slug, slug_from_title |
| `src-tauri/src/slug.rs` | slug_from_title |
| `src-tauri/src/page.rs` | attachment_open |

## Entry Points

Start here when exploring this area:

- **`slug_from_title`** (Function) — `src-tauri/src/slug.rs:3`
- **`attachment_open`** (Function) — `src-tauri/src/page.rs:85`
- **`derive_slug`** (Function) — `core/src/slug.rs:10`
- **`slug_from_title`** (Function) — `core/src/slug.rs:40`
- **`vault_root`** (Function) — `core/src/page.rs:13`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `slug_from_title` | Function | `src-tauri/src/slug.rs` | 3 |
| `attachment_open` | Function | `src-tauri/src/page.rs` | 85 |
| `derive_slug` | Function | `core/src/slug.rs` | 10 |
| `slug_from_title` | Function | `core/src/slug.rs` | 40 |
| `vault_root` | Function | `core/src/page.rs` | 13 |
| `page_create` | Function | `core/src/page.rs` | 29 |
| `page_delete` | Function | `core/src/page.rs` | 133 |
| `attachment_list` | Function | `core/src/page.rs` | 144 |
| `attachment_add` | Function | `core/src/page.rs` | 166 |
| `attachment_upload` | Function | `core/src/page.rs` | 188 |
| `attachment_delete` | Function | `core/src/page.rs` | 207 |
| `attachment_path` | Function | `core/src/page.rs` | 220 |
| `attachment_ops` | Function | `core/src/page.rs` | 385 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Attachment_ops → Chrono_now` | cross_community | 7 |
| `Attachment_open → Chrono_now` | cross_community | 7 |
| `Attachment_delete → Chrono_now` | cross_community | 7 |
| `Page_write → Chrono_now` | cross_community | 6 |
| `Page_list → Chrono_now` | cross_community | 6 |
| `Attachment_ops → Config_dir` | cross_community | 6 |
| `Attachment_open → Config_dir` | cross_community | 6 |
| `Page_read → Chrono_now` | cross_community | 6 |
| `Page_delete → Chrono_now` | cross_community | 6 |
| `Attachment_upload → Chrono_now` | cross_community | 6 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_0 | 1 calls |
| Cluster_4 | 1 calls |

## How to Explore

1. `gitnexus_context({name: "slug_from_title"})` — see callers and callees
2. `gitnexus_query({query: "cluster_1"})` — find related execution flows
3. Read key files listed above for implementation details
