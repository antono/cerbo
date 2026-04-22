## Context

Currently, adding a vault in Cerbo requires manually typing or pasting a filesystem path into a text input. This is prone to errors and provides a suboptimal user experience compared to native OS folder pickers. Tauri v2 provides a dedicated plugin, `tauri-plugin-dialog`, to handle these native interactions securely.

## Goals / Non-Goals

**Goals:**
- Provide a "Browse" button in the Add Vault UI that opens a native folder selection dialog.
- Automatically populate the vault path input with the selected directory.
- Allow creation of new folders within the native dialog.
- Ensure the plugin is correctly registered and permissioned in the Tauri project.

**Non-Goals:**
- Implementing a custom file explorer in Svelte.
- Supporting folder selection on mobile platforms (limited by OS and plugin).
- Handling complex file filters (only directories are needed).

## Decisions

### D1: Use `tauri-plugin-dialog` v2
The official Tauri v2 dialog plugin is the standard way to access native file and folder pickers.

**Rationale:** It's maintained by the Tauri team, provides a clean TypeScript API, and integrates directly into the v2 permission system.
**Alternatives considered:**
- Writing a custom Rust command using the `rfd` crate: This would reinvent what the plugin already does and require more boilerplate.

### D2: Frontend invocation in `VaultSwitcher.svelte`
The dialog will be triggered directly from the `VaultSwitcher` component.

**Rationale:** The folder selection is a transient UI interaction that only needs to populate a local state variable (`newVaultPath`). There is no need for complex state management in a global store for this specific action.

### D3: Capability-based permissions
Access to the native open dialog must be explicitly allowed in the app's capabilities.

**Rationale:** Tauri v2 enforces strict isolation. We will add `dialog:allow-open` to `src-tauri/capabilities/default.json`.

## Risks / Trade-offs

- **[Risk] Path Ambiguity** → Some OS dialogs might return paths with trailing separators or specific encodings.
- **[Mitigation]** The selected path will be passed to the existing `vault_add` command which already performs basic validation.
- **[Risk] Desktop Only** → Native folder selection via this plugin is primarily for desktop.
- **[Mitigation]** Cerbo is currently targeted at desktop platforms, so this is acceptable. Mobile targets would require a different approach (Document Picker APIs).

## Open Questions

- Should we set a `defaultPath` for the dialog (e.g., the user's home directory)?
- **Answer**: Yes, using `@tauri-apps/api/path`'s `homeDir()` provides a better starting point than the current working directory.
