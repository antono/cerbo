## Why

Cerbo has multiple confirmation modals that overlap in purpose but not in interaction model or visual treatment. The quit confirmation is already the clearest pattern, and the delete-page dialog should match it so confirmation flows feel consistent and modal keyboard behavior is predictable.

## What Changes

- Unify modal confirmation dialogs around the current quit confirmation pattern.
- Make delete-page confirmation look and behave like quit confirmation.
- Ensure only `Arrow` keys, `Enter`, and `Escape` are handled while a modal confirmation dialog is active.
- Ignore all other app hotkeys while any modal confirmation dialog is open.
- **BREAKING**: confirmation dialogs will no longer use ad hoc layouts or independent keyboard handling.

## Capabilities

### New Capabilities
- `modal-confirmation-dialogs`: shared confirmation dialog design and keyboard behavior for app confirmation modals.

### Modified Capabilities
- `design-tokens`: confirmation dialog visuals should align with the quit confirmation component’s current treatment.
- `global-hotkeys`: app-level hotkeys must be suppressed while a modal confirmation dialog is active.

## Impact

- `src/lib/ExitConfirmation.svelte`: likely becomes the shared confirmation pattern or a reference implementation.
- `src/lib/PageList.svelte`: delete-page confirmation UI and keyboard handling.
- `src/routes/+layout.svelte`: modal gating for app hotkeys while confirmations are open.
- Shared styling tokens for modal surface, actions, and focus state.
