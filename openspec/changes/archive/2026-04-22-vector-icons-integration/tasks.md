## 1. Setup

- [x] 1.1 Install `lucide-svelte` dependency using `bun install lucide-svelte`.
- [x] 1.2 Verify installation by checking `package.json`.

## 2. Refactor Existing Icons

- [x] 2.1 Replace manual SVG paths in `ThemeToggle.svelte` with `Sun` and `Moon` components from `lucide-svelte`.
- [x] 2.2 Standardize `size` (18px) and `stroke-width` (2px) for the theme toggle icons.

## 3. Implement Icon System in Shell

- [x] 3.1 Replace emoji `🗂` in `+layout.svelte` with the `Library` or `Database` icon.
- [x] 3.2 Replace emoji `⊞` in `VaultSwitcher.svelte` with the `Square` or `Box` icon.
- [x] 3.3 Add `Plus` icon to the "Add Vault" button in `VaultSwitcher.svelte`.
- [x] 3.4 Add `Folder` or `FileText` icons to the page list items in `PageList.svelte`.
- [x] 3.5 Replace emojis `👁` and `✍` in `+page.svelte` with `Eye` and `Pencil` or `FileCode` icons.

## 4. Verification

- [x] 4.1 Verify that icons invert colors correctly when switching between Light and Dark modes.
- [x] 4.2 Run `devenv tasks run frontend:check` to ensure no TypeScript or Svelte errors.
- [x] 4.3 Manually inspect the UI for visual consistency across all icons.
