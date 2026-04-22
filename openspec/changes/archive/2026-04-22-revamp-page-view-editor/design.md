## Context

The current `PageEditor` component manually manages a separate "edit" mode and a "preview" mode. This involves a toggle in the parent `+page.svelte`, a custom title bar, and heavy CSS overrides to force Carta's toolbar into a vertical sidebar. This approach is brittle and leads to broken styling in dark mode.

## Goals / Non-Goals

**Goals:**
- Implement a unified tabbed editor/viewer using Carta's `mode="tabs"`.
- Clean up the page layout to remove the redundant title bar and edit toggle.
- Ensure high-contrast readability and icon visibility in both light and dark themes.
- Leverage Carta's default top-toolbar layout to simplify CSS and improve plugin compatibility.

**Non-Goals:**
- A pixel-perfect clone of the GitHub UI (focus is on the functional tabbed layout).
- Changing the backend file structure or storage logic.

## Decisions

- **Decision 1: Carta Native Tabs.** We will use `mode="tabs"` in `MarkdownEditor`. This removes the need for manual `isEditing` state management in most places and aligns with the user's request for a tabbed experience.
- **Decision 2: Standard Top Toolbar.** We will remove the vertical sidebar toolbar overrides. This restores Carta's natural flow and makes it easier to style the editor across themes.
- **Decision 3: Integrated Title Rendering.** The page title bar in `src/routes/+page.svelte` will be removed. The title will be rendered solely as the first `# Heading` in the markdown preview.
- **Decision 4: Theme Variable Mapping.** We will explicitly map Carta's CSS variables (e.g., `--carta-bg`, `--carta-fg`, `--carta-border`) to our application variables (`--bg`, `--fg`, `--border`) in `src/app.css`, and add specific overrides for dark mode to fix icon visibility and text contrast.

## Risks / Trade-offs

- **[Risk]** The vertical toolbar provided quick access to all tools. **[Mitigation]** The horizontal toolbar at the top is more standard for editors and provides a better writing experience on most screens.
- **[Risk]** Loss of the prominent title bar might make it less clear which page is open. **[Mitigation]** The H1 heading in the markdown preview will serve as the primary title, and the sidebar highlights the active page.
- **[Risk]** Wikilink click handling in preview might conflict with Carta's internal rendering. **[Mitigation]** We will ensure `attachPreviewClickHandler` is properly attached to the element rendered by Carta.
