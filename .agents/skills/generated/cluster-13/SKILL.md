---
name: cluster-13
description: "Skill for the Cluster_13 area of cerbo. 9 symbols across 1 files."
---

# Cluster_13

9 symbols | 1 files | Cohesion: 73%

## When to Use

- Working with code in `src/`
- Understanding how activeVault, openPage, openNextPage work
- Modifying cluster_13-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/lib/stores.svelte.ts` | activeVault, openPage, openNextPage, openPrevPage, renamePage (+4) |

## Entry Points

Start here when exploring this area:

- **`activeVault`** (Function) — `src/lib/stores.svelte.ts:87`
- **`openPage`** (Function) — `src/lib/stores.svelte.ts:216`
- **`openNextPage`** (Function) — `src/lib/stores.svelte.ts:249`
- **`openPrevPage`** (Function) — `src/lib/stores.svelte.ts:256`
- **`renamePage`** (Function) — `src/lib/stores.svelte.ts:302`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `activeVault` | Function | `src/lib/stores.svelte.ts` | 87 |
| `openPage` | Function | `src/lib/stores.svelte.ts` | 216 |
| `openNextPage` | Function | `src/lib/stores.svelte.ts` | 249 |
| `openPrevPage` | Function | `src/lib/stores.svelte.ts` | 256 |
| `renamePage` | Function | `src/lib/stores.svelte.ts` | 302 |
| `loadBacklinks` | Function | `src/lib/stores.svelte.ts` | 349 |
| `goBack` | Function | `src/lib/stores.svelte.ts` | 362 |
| `goForward` | Function | `src/lib/stores.svelte.ts` | 369 |
| `loadAttachments` | Function | `src/lib/stores.svelte.ts` | 376 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `QuickAddVault → ActiveVault` | cross_community | 6 |
| `QuickAddVault → LoadBacklinks` | cross_community | 6 |
| `QuickAddVault → LoadAttachments` | cross_community | 6 |
| `AddVault → SetError` | cross_community | 5 |
| `DeletePage → ActiveVault` | cross_community | 3 |
| `DeletePage → LoadBacklinks` | cross_community | 3 |
| `DeletePage → LoadAttachments` | cross_community | 3 |
| `RenamePage → SetError` | cross_community | 3 |
| `RenamePage → ActiveVault` | intra_community | 3 |
| `RenamePage → LoadBacklinks` | intra_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_12 | 2 calls |

## How to Explore

1. `gitnexus_context({name: "activeVault"})` — see callers and callees
2. `gitnexus_query({query: "cluster_13"})` — find related execution flows
3. Read key files listed above for implementation details
