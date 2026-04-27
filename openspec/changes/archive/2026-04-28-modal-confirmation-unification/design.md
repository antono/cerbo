## Context

Cerbo currently has at least two confirmation surfaces: the quit dialog and the delete-page dialog. They solve the same class of problem but are implemented independently, which creates visual drift and inconsistent keyboard behavior. The quit confirmation already establishes the desired interaction pattern: centered modal, simple two-action layout, and modal-local keyboard control.

The requested change spans the modal UI components and the layout-level hotkey gate. The key constraint is that confirmation dialogs should behave like a true modal state: only navigation/confirmation keys should remain active, and all other app hotkeys must be suppressed while the dialog is open.

## Goals / Non-Goals

**Goals:**
- Make confirmation dialogs use the same visual treatment as the quit confirmation dialog.
- Make delete-page confirmation follow the same keyboard model as quit confirmation.
- Ensure modal confirmation state blocks unrelated global hotkeys.
- Keep the current quit confirmation as the reference pattern.

**Non-Goals:**
- Redesign non-confirmation dialogs like page search or vault selection.
- Change delete semantics or quit behavior.
- Introduce a new dialog framework or external dependency.

## Decisions

- Reuse the quit confirmation as the canonical confirmation pattern instead of creating a new generic dialog framework. This keeps the change small and ensures the user-facing treatment stays aligned across confirm actions.
  - Alternative considered: extract a shared `ConfirmationDialog` component. Rejected for now because the app only has a small number of confirmation surfaces, and the immediate need is consistency rather than abstraction.
- Move modal hotkey suppression into the layout-level key gate, keyed off active confirmation state.
  - Alternative considered: each modal traps only its own keys. Rejected because app-wide shortcuts are already centralized and can still leak through when a modal is open.
- Keep confirmation dialogs using simple button-based selection with arrow/enter/escape behavior.
  - Alternative considered: full focus-trapped listbox semantics. Rejected because the current quit dialog already provides a lightweight interaction model and the goal is unification, not feature expansion.
- Standardize the delete-page dialog markup and styles to match the quit dialog’s surface, spacing, actions, and focus state.

## Risks / Trade-offs

- [Over-generalizing confirmation UI] → Mitigate by keeping the scope limited to confirmation dialogs only and avoiding a broad dialog framework.
- [Hotkey regressions outside confirmations] → Mitigate by preserving the existing global hotkey branches and only gating them when confirmation state is active.
- [Visual mismatch during incremental migration] → Mitigate by updating the delete-page dialog to match the quit dialog in the same change.
- [Accessibility drift] → Mitigate by preserving dialog roles, focus behavior, and Escape handling in both confirmation surfaces.

## Migration Plan

1. Update the delete-page dialog to match the quit confirmation structure and styles.
2. Ensure confirmation modals own keyboard handling while open.
3. Gate layout-level global hotkeys when confirmation state is active.
4. Validate quit, delete-page, and any other confirmation dialogs still dismiss and confirm correctly.

Rollback is straightforward: restore the previous delete-page modal markup and remove the confirmation-state hotkey gate.

## Open Questions

- Should future confirmation dialogs always reuse the quit dialog component directly, or should the current style be extracted into a shared component after this change lands?
- Do we want the confirmation-state hotkey gate to cover only delete/quit, or any modal that uses the confirmation pattern in the future?
