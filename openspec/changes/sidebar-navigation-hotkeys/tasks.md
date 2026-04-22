## 1. Sidebar Navigation Logic

- [ ] 1.1 Add `handleListKeydown` function to `src/lib/PageList.svelte`.
- [ ] 1.2 Implement index-based navigation for `ArrowUp` and `ArrowDown` keys.
- [ ] 1.3 Implement `Tab` key interception for cycling through the page list.
- [ ] 1.4 Implement `j` and `k` navigation with `isInputFocused` check.
- [ ] 1.5 Ensure focus wrapping at the top and bottom of the page list.
- [ ] 1.6 Attach the keydown handler to the sidebar items container.

## 2. Shortcuts UI Update

- [ ] 2.1 Update `src/lib/KeyboardHelp.svelte` to include a new "Sidebar Navigation" section.
- [ ] 2.2 Add descriptions for Arrows, Tab, and j/k shortcuts in the Help modal.

## 3. Verification

- [ ] 3.1 Manually verify `Tab` cycles focus through all pages in the sidebar.
- [ ] 3.2 Manually verify `ArrowUp`/`ArrowDown` move focus and wrap correctly.
- [ ] 3.3 Manually verify `j`/`k` move focus and do not trigger when typing in inputs.
- [ ] 3.4 Confirm new shortcuts are accurately displayed in the F1 Help modal.
