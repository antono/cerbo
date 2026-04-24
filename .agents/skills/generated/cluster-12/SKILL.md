---
name: cluster-12
description: "Skill for the Cluster_12 area of cerbo. 12 symbols across 1 files."
---

# Cluster_12

12 symbols | 1 files | Cohesion: 85%

## When to Use

- Working with code in `src/`
- Understanding how closeAllDialogs, loadVaults, openVault work
- Modifying cluster_12-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/lib/stores.svelte.ts` | closeAllDialogs, loadVaults, openVault, addVault, quickAddVault (+7) |

## Entry Points

Start here when exploring this area:

- **`closeAllDialogs`** (Function) — `src/lib/stores.svelte.ts:74`
- **`loadVaults`** (Function) — `src/lib/stores.svelte.ts:108`
- **`openVault`** (Function) — `src/lib/stores.svelte.ts:126`
- **`addVault`** (Function) — `src/lib/stores.svelte.ts:165`
- **`quickAddVault`** (Function) — `src/lib/stores.svelte.ts:177`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `closeAllDialogs` | Function | `src/lib/stores.svelte.ts` | 74 |
| `loadVaults` | Function | `src/lib/stores.svelte.ts` | 108 |
| `openVault` | Function | `src/lib/stores.svelte.ts` | 126 |
| `addVault` | Function | `src/lib/stores.svelte.ts` | 165 |
| `quickAddVault` | Function | `src/lib/stores.svelte.ts` | 177 |
| `loadPages` | Function | `src/lib/stores.svelte.ts` | 202 |
| `savePage` | Function | `src/lib/stores.svelte.ts` | 263 |
| `createPage` | Function | `src/lib/stores.svelte.ts` | 273 |
| `deletePage` | Function | `src/lib/stores.svelte.ts` | 282 |
| `triggerRename` | Function | `src/lib/stores.svelte.ts` | 317 |
| `triggerDelete` | Function | `src/lib/stores.svelte.ts` | 328 |
| `setError` | Function | `src/lib/stores.svelte.ts` | 392 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `QuickAddVault → SetError` | intra_community | 6 |
| `QuickAddVault → ActiveVault` | cross_community | 6 |
| `QuickAddVault → LoadBacklinks` | cross_community | 6 |
| `QuickAddVault → LoadAttachments` | cross_community | 6 |
| `AddVault → SetError` | cross_community | 5 |
| `DeletePage → SetError` | intra_community | 3 |
| `DeletePage → ActiveVault` | cross_community | 3 |
| `DeletePage → LoadBacklinks` | cross_community | 3 |
| `DeletePage → LoadAttachments` | cross_community | 3 |
| `RenamePage → SetError` | cross_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_13 | 4 calls |

## How to Explore

1. `gitnexus_context({name: "closeAllDialogs"})` — see callers and callees
2. `gitnexus_query({query: "cluster_12"})` — find related execution flows
3. Read key files listed above for implementation details
