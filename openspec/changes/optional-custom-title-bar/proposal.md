## Why

On Wayland, Tauri applications cannot control window border/decorations colors through APIs - the window manager (KWin, GNOME Shell, etc.) controls this. When users toggle themes in the app, only the webview content theme changes, while window borders remain stuck in the system's default styling. This is a fundamental Wayland limitation (Tauri issue #1046). Building a custom title bar gives the app full control over its window appearance, matching the theme dynamically.

## What Changes

- Add optional custom title bar with window controls (close, minimize, maximize)
- Theme-aware styling - border/bottom bar adapts to light/dark theme
- User toggle to switch between custom title bar and native decorations
- Preserve native decorations as default (backwards compatible)

## Capabilities

### New Capabilities
- `custom-title-bar`: Optional custom window title bar with theme-aware styling and window controls
  - Toggle to enable/disable custom title bar
  - Theme synchronization (light/dark adapts title bar colors)
  - Window controls: close, minimize, maximize
  - Draggable title bar area for window movement

### Modified Capabilities
- `theme-management`: Add specification for how custom title bar responds to theme changes
- `application-lifecycle`: Handle title bar initialization on app startup

## Impact

- **Frontend**: New Svelte component for custom title bar
- **Configuration**: `tauri.conf.json` updates for decorations setting
- **Theme sync**: Modify existing theme sync code to also update custom title bar when toggled