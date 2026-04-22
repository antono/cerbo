## 1. Styling and Theme Cleanup

- [x] 1.1 Remove vertical toolbar overrides and aggressive Carta styling from `src/app.css`.
- [x] 1.2 Map Carta CSS variables to application theme variables in `src/app.css`.
- [x] 1.3 Add dark-mode specific overrides for Carta icons (SVG fill/stroke) and text contrast.
- [x] 1.4 Implement basic tab styling for Carta's "Write" and "Preview" buttons to match a GitHub-like layout.

## 2. Editor Component Refactor

- [x] 2.1 Update `src/lib/PageEditor.svelte` to use `MarkdownEditor` in `mode="tabs"`.
- [x] 2.2 Remove manual `isEditing` state and associated prop from `PageEditor`.
- [x] 2.3 Integrate `wikilinkPlugin` click handlers with Carta's internal preview renderer.
- [x] 2.4 Verify that auto-save continues to function correctly in the new tabbed mode.

## 3. Page Layout Integration

- [x] 3.1 Remove the title bar and manual mode-toggle button from `src/routes/+page.svelte`.
- [x] 3.2 Refactor `+page.svelte` to accommodate the unified editor and backlinks panel.
- [x] 3.3 Ensure the save indicator is correctly positioned in the new layout.

## 4. Verification

- [x] 4.1 Test the tabbed interface in both light and dark themes.
- [x] 4.2 Confirm that page titles (H1 headings) render correctly in the preview tab.
- [x] 4.3 Verify that wikilinks in the preview tab remain interactive and navigate correctly.
- [x] 4.4 Ensure no regressions in the backlinks panel or sidebar functionality.
