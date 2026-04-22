## Context

The current application lacks comprehensive keyboard shortcut support, forcing users to rely on mouse interactions for core tasks like switching between editor modes, searching for pages, and navigating between panels.

## Goals / Non-Goals

**Goals:**
- Implement global shortcuts for page search (`Ctrl+P`) and application exit (`Ctrl+Q`).
- Implement context-sensitive shortcuts for editor mode switching (`Ctrl+I`, `i`, `Esc`).
- Implement panel navigation (`Ctrl+Arrows`).
- Ensure native behavior on macOS by mapping `Ctrl` to `Cmd`.
- Provide a keyboard-navigable confirmation dialog for quitting.

**Non-Goals:**
- Custom hotkey configuration (remapping) by the user.
- Hotkeys for all sidebar actions (e.g., individual page rename/delete).

## Decisions

### 1. Centralized Shortcut Normalization
**Decision**: Create a `src/lib/hotkeys.ts` utility to normalize keyboard events.
**Rationale**: Simplifies platform-specific logic (Mac `Cmd` vs. Windows/Linux `Ctrl`) and provides a single place to define "is this a shortcut?" checks.

### 2. App Store State for UI Toggles
**Decision**: Extend the `app` store in `src/lib/stores.svelte.ts` with:
- `editorMode`: `'write' | 'preview'`
- `activePanel`: `'sidebar' | 'editor' | 'panels'`
- `showSearch`: `boolean`
- `showExitPrompt`: `boolean`
**Rationale**: Svelte 5 runes allow reactive synchronization across disjoint components (`PageEditor`, `PageList`, `+layout`).

### 3. Global Event Delegation
**Decision**: Attach a single `onkeydown` listener to the window in `src/routes/+layout.svelte`.
**Rationale**: Efficiently captures global shortcuts like `Ctrl+P` and `Ctrl+Q` regardless of where focus is, while allowing local overrides (e.g., inside an input) by checking `event.target`.

### 4. Panel Navigation via Focus Management
**Decision**: Use `app.activePanel` to determine which DOM element should receive focus when `Ctrl+Arrows` is pressed. Panels will have `tabindex="-1"` and focusable wrappers.
**Rationale**: Provides a predictable "focus flow" across the three-column layout.

### 5. Exit Confirmation via Svelte Modal
**Decision**: Use a custom Svelte modal instead of native `window.confirm`.
**Rationale**: Matches application styling and allows for custom keyboard navigation (Arrow keys/Enter) as required by the specs.

## Risks / Trade-offs

- **[Risk] Collision with Browser/OS Shortcuts** → **Mitigation**: Use `preventDefault()` for handled shortcuts and stick to standard mappings.
- **[Risk] Focus Trap in Editor** → **Mitigation**: Ensure `Esc` and `Ctrl+Arrows` escape the Carta editor instance by listening at the window level during the capture phase if necessary.
- **[Risk] Accidental Quits** → **Mitigation**: The mandatory confirmation dialog protects against unintended closures.
