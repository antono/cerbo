## 1. Foundation

- [x] 1.1 Create `src/lib/hotkeys.ts` utility for platform-aware keyboard event normalization
- [x] 1.2 Extend `app` store in `src/lib/stores.svelte.ts` with `editorMode`, `activePanel`, `showSearch`, and `showExitPrompt` states
- [x] 1.3 Implement `quitApp` command in `src/lib/stores.svelte.ts` using Tauri's exit API

## 2. UI Components

- [x] 2.1 Implement `src/lib/GlobalSearch.svelte` modal with fuzzy matching and keyboard navigation
- [x] 2.2 Implement `src/lib/ExitConfirmation.svelte` modal with Arrow/Enter/Esc navigation
- [x] 2.3 Refactor `src/lib/PageEditor.svelte` to use centralized `app.editorMode` state

## 3. Integration & Focus Management

- [x] 3.1 Register global `onkeydown` handler in `src/routes/+layout.svelte`
- [x] 3.2 Implement focus management logic for `Ctrl+Arrows` panel switching
- [x] 3.3 Add `tabindex="-1"` and focus styles to major layout containers
- [x] 3.4 Ensure `Ctrl+P` and `Ctrl+Q` triggers work globally

## 4. Mode-Specific Shortcuts

- [x] 4.1 Implement `Ctrl+I` and `i` (preview mode) switching to edit mode
- [x] 4.2 Implement `Esc` (edit mode) switching to preview mode
- [x] 4.3 Ensure `i` shortcut is ignored when focusing input elements

## 5. Verification

- [x] 5.1 Verify search modal opens and navigates correctly via keyboard
- [x] 5.2 Verify exit dialog prevents accidental closure and confirms quit via keyboard
- [x] 5.3 Verify panel focus switching cycles correctly between Sidebar, Editor, and Side Panels
