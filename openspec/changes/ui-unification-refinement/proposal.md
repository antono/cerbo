## Why
The current UI lacks consistency in spacing, borders, and component alignment, leading to a "fragmented" feel. The layout is static, and specific components like the Vault Switcher and Page Editor have visual bugs (wrapping text, double scrollbars) that hinder user experience. Standardizing the UI and introducing resizable panels will provide a professional, modern productivity tool feel aligned with `shadcn-svelte` aesthetics.

## What Changes
- **Unified Variables**: Establish a consistent design language via global CSS variables for spacing, headers, borders, and radius.
- **Resizable Layout**: Introduce a 3-column layout with resizable left sidebar (navigation) and right panel (backlinks).
- **Aligned Toolbars**: Synchronize the height of the Vault Selector and Page Editor Toolbar for a clean, continuous horizontal line.
- **Enhanced Button UI**: "Bump" the Add Vault button with primary coloring and fixed flex layout to prevent icon/text wrapping.
- **Carta Styling Fixes**: Synchronize `carta-md` borders/radius with the app's theme and eliminate double scrollbars in the editor.

## Capabilities

### New Capabilities
- `resizable-layout`: Core layout system supporting drag handles and panel width persistence.
- `panel-management`: Toggle visibility and state for side panels.

### Modified Capabilities
- `backlinks`: Requirements change to support right-side positioning and visibility control.
- `vault-management`: UI refinement for the vault selection process.
- `theme-management`: Refinement of component styling consistency across the app.

## Impact
- `src/app.css`: Central theme definition.
- `src/routes/+layout.svelte`: Core layout structural changes.
- `src/lib/PageEditor.svelte`: Editor alignment and scroll fixes.
- `src/lib/VaultSwitcher.svelte`: Vault button redesign.
- `src/lib/BacklinksPanel.svelte`: Panel repositioning.
