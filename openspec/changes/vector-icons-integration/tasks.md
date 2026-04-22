## 1. Setup

- [ ] 1.1 Install `lucide-svelte` dependency using `npm install lucide-svelte`.
- [ ] 1.2 Verify installation by checking `package.json`.

## 2. Refactor Existing Icons

- [ ] 2.1 Replace manual SVG paths in `ThemeToggle.svelte` with `Sun` and `Moon` components from `lucide-svelte`.
- [ ] 2.2 Standardize `size` (18px) and `stroke-width` (2px) for the theme toggle icons.

## 3. Implement Icon System in Shell

- [ ] 3.1 Replace emoji `🗂` in `+layout.svelte` with the `Library` or `Database` icon.
- [ ] 3.2 Replace emoji `⊞` in `VaultSwitcher.svelte` with the `Square` or `Box` icon.
- [ ] 3.3 Add `Plus` icon to the "Add Vault" button in `VaultSwitcher.svelte`.
- [ ] 3.4 Add `Folder` or `FileText` icons to the page list items in `PageList.svelte`.

## 4. Verification

- [ ] 4.1 Verify that icons invert colors correctly when switching between Light and Dark modes.
- [ ] 4.2 Run `devenv tasks run frontend:check` to ensure no TypeScript or Svelte errors.
- [ ] 4.3 Manually inspect the UI for visual consistency across all icons.
