## Context

The sidebar page list is implemented in `src/lib/PageList.svelte`. It uses Svelte 5 runes and accesses the global `app` store for the list of pages. Each page is represented by a `<button class="page-btn">`. Currently, there is no specialized keyboard navigation logic beyond standard browser tab behavior.

## Goals / Non-Goals

**Goals:**
- Enable efficient list navigation using `ArrowUp`, `ArrowDown`, `Tab`, `j`, and `k`.
- Ensure focus stays within the page list when cycling with `Tab` (optional but preferred by user request).
- Support wrapping from the end of the list back to the beginning.

**Non-Goals:**
- Implementing a full focus-trap for the sidebar (focus can still leave via other keys if not handled).
- Changing how pages are opened (still via the existing `openPage` function).

## Decisions

### 1. Centralized Keydown Handler in PageList.svelte
Add a `handleListKeydown` function to the `<ul>` element that contains the page items. This allows us to use event delegation or direct handling on focused buttons.

**Rationale:** Keeping navigation logic close to the data (`app.pages`) simplifies index calculations.

### 2. Manual Focus Management
We will use `querySelectorAll('.page-btn')` or a reactive list of refs to find the next/previous button to focus.

**Rationale:** Standard `focus()` on the button element is the most reliable way to ensure accessibility and visual feedback.

### 3. j/k Handling
Vim-style navigation keys will only be active if `!isInputFocused()` is true, to avoid interfering with typing in the "New Page" or "Rename" fields.

### 4. Tab Interception
When `Tab` is pressed on a page button, we will prevent the default behavior and manually move focus to the next item (with wrapping).

**Rationale:** The user specifically asked for `Tab` to move to the "next page", which deviates from standard "leave the list" behavior.

## Risks / Trade-offs

- **[Risk] Conflict with browser shortcuts** → **Mitigation**: Standard keys like `Tab` and `Arrows` are only intercepted when focus is specifically on a sidebar page item.
- **[Risk] Syncing focus with active page** → **Mitigation**: Ensure that when a page is opened via a shortcut, it also receives the `active` class (already handled by store).
