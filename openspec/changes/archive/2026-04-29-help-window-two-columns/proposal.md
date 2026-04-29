## Why

The keyboard shortcuts help modal is currently too narrow and rendered as a single-column list, which makes it harder to scan on wider screens. A wider modal with a two-column layout on large viewports improves readability and uses available space better without changing the help content.

## What Changes

- Widen the shortcuts help modal shell for large screens.
- Update the shortcuts help modal to render in two columns on wide screens.
- Preserve a single-column layout on narrow screens.
- Split shortcuts by meaning so related actions stay grouped together.
- Keep existing close behavior and shortcut content unchanged.

```
narrow                                   wide
┌──────────────────────┐                 ┌──────────────────────────────┐
│ Keyboard Shortcuts   │                 │ Keyboard Shortcuts           │
├──────────────────────┤                 ├──────────────────────────────┤
│ App actions          │                 │ App actions   │ Navigation   │
│ - Ctrl+P Open search │                 │ - Ctrl+P ...  │ - Alt+← ...  │
│ - Ctrl+N New page    │                 │ - Ctrl+N ...  │ - Alt+→ ...  │
│ - F1 Help            │                 │ - F1 ...      │ - j/k ...    │
│                      │                 │               │ - Delete ... │
└──────────────────────┘                 └──────────────────────────────┘
```

## Capabilities

### New Capabilities

- `shortcuts-help-layout`: responsive multi-column layout for the shortcuts help modal.

### Modified Capabilities

- `shortcuts-help`: update the help modal presentation to support a responsive two-column layout while preserving existing content and dismissal behavior.

## Impact

Affected code is limited to the help modal component and its associated spec/artifacts. The change should not affect keyboard handling, routing, or other modal behavior.
