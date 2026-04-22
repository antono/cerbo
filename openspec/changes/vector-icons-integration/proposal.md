## Why
The app currently uses a mix of emojis and manual SVG paths for icons, which is inconsistent and difficult to maintain. Adding a standard, minimal vector icon library (Lucide) will provide a polished, professional visual identity that automatically supports dark/light themes.

## What Changes
- Add `lucide-svelte` as a dependency.
- Replace hardcoded SVG paths in `ThemeToggle.svelte` with Lucide components.
- Replace placeholder emojis (like `🗂` and `⊞`) in `VaultSwitcher.svelte` and the sidebar with consistent Lucide icons.
- Establish `lucide-svelte` as the project's standard icon system.

## Capabilities

### New Capabilities
- `icon-system`: Standards and patterns for using vector icons throughout the app.

### Modified Capabilities
- `theme-toggle-component`: Update the toggle to use components from the new icon system.

## Impact
- **Dependencies**: New `lucide-svelte` package.
- **UI Components**: `ThemeToggle.svelte`, `VaultSwitcher.svelte`, and `+layout.svelte`.
- **Consistency**: Unified stroke-based visual style across all UI elements.
