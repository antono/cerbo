# Design

## Decisions

### D1. Reuse core, no second pipeline
`plan_import(source, Some(format))` is the dry run, `run_import(ctx, plan,
ImportOptions{force})` the real run. The commands only translate arguments and
errors, exactly as `run_import_vault` in the CLI does. Order of checks mirrors
the CLI so every failure precedes the first write: target outside source →
plan → (create + register new vault) → run.

### D2. Two commands, stateless
- `vault_import_plan(source, format) -> ImportReport`: runs on
  `spawn_blocking`, writes nothing.
- `vault_import_run(source, format, target, force) -> ImportReport` where
  `target = {existing: id} | {new: {path, name}}`. It re-plans: UUIDs in the
  plan are random, but nothing user-visible depends on the preview's UUIDs,
  and holding a plan in app state would leak memory for abandoned dialogs.

Rejected: caching the plan between commands (state + invalidation if the
source changes between preview and import).

### D3. Format step before folder step
The user's mental model is "I use Obsidian". The choice sets `from`, so
`detect` only has to *confirm*: result `Detection{format, reason}` from
`detect(dir, None)` is compared with the choice; a mismatch shows "This looks
like a Tolaria vault. Switch to Tolaria?".

### D4. New vault is the default target
Importing into a fresh vault is the safe case (nothing to collide with).
New vault: `check_new_vault_dir` and `vault_create` (as `cerbo init`) run only
after the plan succeeded, same as the CLI.

### D5. Errors and long runs
Core errors are strings; they are shown inline in the dialog, which stays open
(no loss of choices). While running, Esc/backdrop are inert. If the app is
closed mid-run, `finish_interrupted_import` (already called on vault open)
takes the half-written objects back.

## UI sketch

```
Import vault                                   esc
 1 Source  2 Folder  3 Target  4 Preview
 ┌───────────────┐ ┌───────────────┐
 │  Obsidian     │ │  Tolaria      │
 └───────────────┘ └───────────────┘
 Folder:  ~/Notes/Obsidian  [Choose…]   ✓ Obsidian (.obsidian/)
 Into:    ● New vault  [name][folder]   ○ <registered vaults>
 Preview: 137 pages · 12 attachments · 3 unresolved links
          [ ] Import again anyway           [ Import ]
```
