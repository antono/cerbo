## Context

The current preview mode uses `j`/`k` for page navigation (next/previous page). This conflicts with vim-style muscle memory where `j`/`k` scroll within content. The sidebar navigation needs its own binding, and vault shortcuts need to match standard patterns (Ctrl+O to open/select, Ctrl+Shift+O to add).

Current state:
- `j`/`k` in preview = navigate pages
- `Ctrl+Shift+O` = Select vault
- `Ctrl+O` = Add vault

## Goals / Non-Goals

**Goals:**
- `j`/`k` scroll preview viewport (vim-style)
- `J`/`K` navigate sidebar page list (next/previous)
- Swap vault shortcuts: `Ctrl+O` = Select, `Ctrl+Shift+O` = Add
- Maintain cross-platform key mapping (Ctrl→Cmd on Mac)

**Non-Goals:**
- Hotkey customization (out of scope)
- New UI for shortcut help (update existing only)
- Changing other existing shortcuts

## Decisions

**1. Use `J`/`K` for sidebar navigation (not `Shift+J`/`Shift+K`)**
- Rationale: Single keystroke is faster; uppercase is conventional "shifted" version of j/k
- Alternatives considered: `Shift+j`/`Shift+k` (requires chord detection), `[`/`]` (less discoverable)

**2. Scroll via `scrollBy` API rather than focus-based navigation**
- Rationale: Preserves scroll position accuracy; works with any scrollable container
- Alternatives considered: Focus management on page elements (complex, brittle)

**3. Swap vault shortcuts to match user mental model**
- Rationale: `Ctrl+O` is universally "Open" (vault selection = opening a vault); `Ctrl+Shift+O` = "Open new" (add vault)
- Alternatives considered: Keep current mapping (confusing to users), use different keys entirely (more disruption)

## Risks / Trade-offs

- **[Risk] Users accustomed to old `j`/`k` navigation may be confused** → Mitigation: Update help modal immediately; consider temporary toast notification on first preview mode entry
- **[Risk] `J`/`K` may conflict with text input if not properly gated** → Mitigation: Check `document.activeElement` to ensure no input/textarea is focused
- **[Risk] Scroll amount may feel too fast/slow** → Mitigation: Use `scrollBy` with configurable pixel amount (start with 100px)
