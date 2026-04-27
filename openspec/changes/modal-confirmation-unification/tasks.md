## 1. Shared confirmation pattern

- [ ] 1.1 Align the delete-page confirmation dialog structure and styling with the current quit confirmation dialog.
- [ ] 1.2 Reuse the quit confirmation keyboard model for confirmation dialogs, including arrow, enter, and escape handling.
- [ ] 1.3 Ensure confirmation dialogs manage focus consistently when they open.

## 2. Hotkey isolation

- [ ] 2.1 Gate layout-level global hotkeys while a modal confirmation dialog is open.
- [ ] 2.2 Verify that only the confirmation dialog's own navigation keys are processed while it is active.
- [ ] 2.3 Confirm non-confirmation global hotkeys still work when no modal confirmation dialog is open.

## 3. Verification

- [ ] 3.1 Validate the updated confirmation dialogs against the quit dialog visual pattern.
- [ ] 3.2 Run the desktop build after the modal changes.
