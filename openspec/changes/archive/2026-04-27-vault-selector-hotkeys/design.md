## Context

Cerbo currently exposes vault switching through an inline sidebar header control, while page selection already uses a modal selector pattern. The request is to move vault switching to a keyboard-first selector, remove the old switcher UI, and surface keyboard help from the main shell chrome.

The change spans layout-level input handling, shared UI state, and two existing modal-style surfaces. The goal is to keep the implementation consistent with the app's current interaction model rather than introduce a new navigation paradigm.

## Goals / Non-Goals

**Goals:**
- Open a vault selector with `Ctrl+Shift+O`.
- Make the vault selector feel like the existing page selector.
- Remove the old inline vault switcher from the sidebar header.
- Add a help icon near theme switching that opens hotkeys help.
- Update help content so the new shortcut is discoverable.

**Non-Goals:**
- Redesign vault management flows beyond selection and add-vault.
- Change the underlying vault open behavior or persistence model.
- Introduce a new command palette architecture.

## Decisions

- Reuse the existing modal/dialog pattern for the vault selector instead of building a separate popover. This keeps keyboard behavior, dismissal, and focus handling aligned with the page search dialog.
- Keep vault selection in `src/lib/stores.svelte.ts` as shared UI state rather than introducing a new store or context layer. The shell already coordinates modal visibility there, so this keeps ownership centralized.
- Update the shell header to host a dedicated help icon next to theme controls instead of burying hotkeys inside menus. This makes the help entry point obvious without affecting page navigation.
- Keep the old `VaultSwitcher.svelte` removal localized to the layout chrome. The page list and vault open flow should continue to use `openVault()` unchanged.

## Risks / Trade-offs

- [Overlapping modal state] → Mitigated by reusing `closeAllDialogs()` before opening the vault selector or help modal.
- [Keyboard shortcut conflicts] → Mitigated by handling the new shortcut at the layout level and bailing out when inputs are focused or other modals are open.
- [UI inconsistency] → Mitigated by matching the vault selector's structure and interaction rules to the existing page selector.
- [Removed switcher discoverability] → Mitigated by adding the help icon and documenting the new shortcut in hotkeys help.

## Migration Plan

1. Add shared state and modal wiring for vault selector/help.
2. Replace the sidebar vault switcher trigger with the new selector entry point.
3. Add the new keyboard shortcut and update shortcut help content.
4. Remove the old vault switcher component usage once the new selector is in place.

Rollback is straightforward: restore the old sidebar switcher wiring and remove the new selector/help entry points.

## Open Questions

- Should the vault selector support a search field from day one, or start with a simple list like the current vault switcher?
- Should the help icon open the same `KeyboardHelp` modal or a lighter-weight popover?
