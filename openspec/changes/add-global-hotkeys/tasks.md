## 1. Store and Utility Updates

- [ ] 1.1 Add `showHelp` state to `app` store in `src/lib/stores.svelte.ts`.
- [ ] 1.2 Update `closeAllDialogs` in `src/lib/stores.svelte.ts` to include `app.showHelp = false`.
- [ ] 1.3 Move `quickAddVault` logic from `src/lib/VaultSwitcher.svelte` to `src/lib/stores.svelte.ts`.
- [ ] 1.4 Refactor `src/lib/VaultSwitcher.svelte` to use the centralized `quickAddVault` command.

## 2. UI Components

- [ ] 2.1 Create `src/lib/KeyboardHelp.svelte` component for displaying the shortcuts reference modal.
- [ ] 2.2 Register and render `KeyboardHelp` component in `src/routes/+layout.svelte`.

## 3. Global Hotkey Implementation

- [ ] 3.1 Implement `Ctrl+T` (Cmd+T) for theme toggling in `src/routes/+layout.svelte`.
- [ ] 3.2 Implement `F1` for opening the Shortcuts Help modal in `src/routes/+layout.svelte`.
- [ ] 3.3 Implement `Ctrl+O` (Cmd+O) for triggering `quickAddVault` in `src/routes/+layout.svelte`.

## 4. Specification Updates

- [ ] 4.1 Apply delta specs to main specification files.
