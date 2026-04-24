## Context

The Cerbo desktop app uses Tauri v2 with Svelte 5 frontend. Currently, theme changes sync to the Tauri webview but cannot control window border colors on Wayland due to API limitations. Users want theme-aware window styling.

**Current state:**
- `tauri.conf.json` uses default native decorations
- Theme sync in `+layout.svelte` calls `setTheme()` and `win.setTheme()` 
- No custom title bar - relies entirely on native window decorations

**Constraints:**
- Must work on Windows too (not just Wayland)
- Keep native decorations as default for backwards compatibility
- Window controls must work - close, minimize, maximize
- Title bar should be draggable for window movement

## Goals / Non-Goals

**Goals:**
- Add custom title bar as optional feature
- Theme-aware styling (adapts to light/dark mode)
- Window controls (close, minimize, maximize) functional
- Toggle in settings to switch between custom and native decorations

**Non-Goals:**
- Not every native decoration feature (just basics)
- Not removing native decorations entirely - toggle only
- Not supporting every Tauri platform - focus on desktop

## Decisions

### Decision 1: Custom title bar implementation approach

**Chosen:** React-component style Svelte 5 component with CSS-based styling
- **Rationale:** Svelte 5 has excellent CSS custom property support, easy theme variables integration
- **Alternative:** HTML/CSS with inline styles - more verbose, harder to maintain

### Decision 2: Window control APIs

**Chosen:** Use `@tauri-apps/api/window` - `getCurrentWindow().close()`, `minimize()`, `toggleMaximize()`
- **Rationale:** Official Tauri v2 API, cross-platform, well-supported
- **Alternative:** Custom IPC to Rust backend - adds complexity, no benefit

### Decision 3: Decoration mode storage

**Chosen:** Store in localStorage with key `cerbo:useCustomTitleBar`
- **Rationale:** Simple, persists across sessions, no backend needed
- **Alternative:** Backend storage - overkill for this preference

### Decision 4: Theme integration

**Chosen:** Reuse existing `mode-watcher` store, add effects for title bar update
- **Rationale:** Avoids duplicating theme logic, integrates with existing theme toggle
- **Alternative:** Separate theme state for title bar - would get out of sync

## Risks / Trade-offs

**[Risk] Custom title bar loses system integrations** → Mitigation: Keep native as default, document that custom mode lacks some system integrations

**[Risk] Theme flash on startup** → Mitigation: Render title bar immediately in onMount before theme resolves

**[Risk] Window drag not working on some platforms** → Mitigation: Use Tauri API `startDragging()` with fallback

**[Risk] Mobile/macOS specific behavior** → Mitigation: Conditionally render custom title bar only on desktop Linux/Windows

## Open Questions

- Should the custom title bar include a menu bar? (defer to future change)
- How to handle traffic lights on macOS? (out of scope for this change)