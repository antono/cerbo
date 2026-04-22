## Why

Enhance user productivity and application ergonomics by introducing standard keyboard shortcuts for common navigation and editing tasks. This reduces reliance on mouse interactions for frequent operations like page switching, mode toggling, and application exit.

## What Changes

- **Page Search**: `Ctrl+P` (Linux/Windows) or `Cmd+P` (Mac) to trigger the page autocomplete/search interface.
- **Edit Mode Toggle**:
    - `Ctrl+I` (Linux/Windows) or `Cmd+I` (Mac) to enter edit mode.
    - Single key `i` when in preview mode (and not focusing an input) to enter edit mode.
    - `Esc` when in edit mode to return to preview mode.
- **Application Exit**: `Ctrl+Q` (Linux/Windows) or `Cmd+Q` (Mac) to quit the application, preceded by a confirmation dialog navigable via keyboard (Arrows/Enter).
- **Panel Navigation**: `Ctrl+Arrows` (Linux/Windows) or `Cmd+Arrows` (Mac) to shift focus between UI panels (e.g., Page List, Main Editor, Sidebar panels).

## Capabilities

### New Capabilities
- `keyboard-shortcuts`: Centralized management and registration of global and context-sensitive keyboard commands.
- `application-lifecycle`: Handles application-level events like quitting with verification.

### Modified Capabilities
- `page-crud`: Integration of edit/preview mode transitions via keyboard.

## Impact

- **UI Components**: `PageEditor`, `PageList`, and the main layout will need to handle focus states and local hotkeys.
- **Tauri/Backend**: The quit confirmation logic may involve Tauri's window APIs and a custom Svelte modal.
- **State Management**: Focus state management across the application to facilitate panel switching.
