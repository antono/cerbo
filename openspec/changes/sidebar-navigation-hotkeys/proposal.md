## Why

To improve keyboard-driven efficiency, users need a way to navigate the page list in the sidebar without relying on the mouse. Adding standard navigation keys (Arrows, Tab) and vim-style shortcuts (j, k) makes the application feel more responsive and accessible to power users.

## What Changes

- **Tab Navigation**: Pressing `Tab` while focus is in the sidebar moves focus to the next page in the list.
- **Arrow Navigation**: `ArrowUp` and `ArrowDown` move focus between pages in the sidebar.
- **Vim-style Navigation**: `j` (down) and `k` (up) move focus between pages when no input is active.
- **Shortcuts Help**: Update the help modal to include these new sidebar navigation shortcuts.

## Capabilities

### New Capabilities
- `sidebar-navigation`: Keyboard navigation logic for the page list, including focus management and wrapping.

### Modified Capabilities
- `keyboard-shortcuts`: Extend the existing shortcut system to include contextual sidebar hotkeys and update the help registry.

## Impact

- `src/lib/PageList.svelte`: Primary location for keydown handling and focus management.
- `src/lib/KeyboardHelp.svelte`: UI update to display the new shortcuts.
- `src/lib/hotkeys.ts`: Potentially add helpers for list navigation if reusable.
