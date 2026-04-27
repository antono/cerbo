# Global Hotkeys

## Purpose
Define how application-level hotkeys behave when transient UI is active.

## Requirements

### Requirement: Confirmation modal hotkey suppression
When a modal confirmation dialog is open, the application MUST suppress all global hotkeys except the confirmation dialog's own navigation and dismissal keys.

### Requirement: Confirmation modal active-state gating
The application MUST treat confirmation modal open state as a blocking condition for layout-level shortcut handling.

### Requirement: Non-confirmation shortcuts preserved
Global hotkeys outside modal confirmation state MUST continue to work unchanged.
