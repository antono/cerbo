## Why

The current page view and editor separation (with an explicit edit button) feels clunky compared to modern tabbed markdown editors (like GitHub's). Additionally, the existing theme support for the editor is broken in dark mode, where icons are hidden and text contrast is poor.

## What Changes

- Replace the manual edit mode toggle and separate page title with a unified tabbed editor (Carta).
- Adopt a "Write" and "Preview" tabbed layout similar to GitHub.
- Render the page title directly from the markdown content (the first `# ` heading) in the preview tab, instead of a separate header bar.
- Improve dark mode styling for Carta icons and editor text.
- Consolidate the main page layout to maximize the editing area while keeping the backlinks panel.

## Capabilities

### New Capabilities
- None

### Modified Capabilities
- `page-crud`: Update requirements to reflect the unified tabbed view and content-driven title rendering.
- `theme-management`: Fix dark mode support for the editor component.

## Impact

- `src/lib/PageEditor.svelte`: Main component to be refactored to `mode="tabs"`.
- `src/routes/+page.svelte`: Layout cleanup to remove separate title bar and edit toggle.
- `src/app.css`: Styling fixes for Carta dark mode and layout adjustments.
