## Context

The shortcuts help modal currently renders as a single vertical list inside a narrow shell. The content is already stable; the requested change is presentational and should make the dialog wider and easier to scan on larger screens without changing the shortcut set or dismissal behavior.

## Goals / Non-Goals

**Goals:**
- Show the help content in two columns on wide screens.
- Make the modal shell much wider on large screens.
- Preserve a single-column layout on narrow screens.
- Keep shortcut content, keyboard handling, and modal close behavior unchanged.
- Group shortcuts by meaning so related actions stay together.

**Non-Goals:**
- No new shortcuts or content changes.
- No changes to global hotkey handling or dialog lifecycle.
- No redesign of other modals.

## Decisions

### Use a responsive grid inside a wider modal
The modal shell should be widened substantially, while the shortcuts body should switch from a single list to a two-column layout at a wide-screen breakpoint. A grid is the simplest fit because it keeps the content flow predictable and collapses cleanly to one column.

Alternatives considered:
- Two separate lists with duplicated row markup: too much markup noise for a small layout change.
- CSS columns: visually simple, but awkward for semantic grouping and keyboard/DOM order.

### Group shortcuts by semantic category
The list should be split into logical sections such as app actions and navigation/page actions. This makes the two-column layout meaningful instead of purely spatial.

Alternatives considered:
- Split by list order: easier to implement, but weaker for scanning and comprehension.
- Hardcode left/right columns independently: flexible, but more brittle if items change later.

### Keep the current row presentation
Each shortcut row should retain the existing key-chord + description pattern. This minimizes visual churn and reduces regression risk.

## Risks / Trade-offs

- [Content growth] → If the list grows, one column may become much longer than the other. Mitigation: use a breakpoint that only enables two columns when there is enough horizontal space.
- [Uneven group sizes] → One semantic group may be noticeably larger than the other. Mitigation: choose group boundaries based on readability first, not perfect balance.
- [Responsive regression] → The new wider modal could crowd smaller laptops. Mitigation: cap the width at a conservative maximum and verify the narrow layout remains unchanged.

## Migration Plan

No migration is required. The change is isolated to the help modal presentation and can be rolled out directly. If the new layout is unsatisfactory, rollback is a simple revert of the help modal styling and grouping markup.

## Open Questions

- What breakpoint should trigger the two-column layout?
- Should preview-mode page actions live in the navigation group or in their own group?
