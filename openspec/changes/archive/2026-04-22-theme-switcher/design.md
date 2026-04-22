## Context

The application currently relies on the system's color scheme preference via `@media (prefers-color-scheme: dark)`. This lacks a manual override for users. The project uses Svelte 5, Tailwind CSS 4, and has `mode-watcher` in its dependencies.

## Goals / Non-Goals

**Goals:**
- Implement a manual light/dark theme switcher.
- Persist the user's theme choice across sessions.
- Integrate `mode-watcher` for theme management.
- Update CSS to support class-based theme switching (manual override).

**Non-Goals:**
- Implementing multiple color themes beyond Light and Dark.
- Fine-grained theme settings (e.g., custom accent colors).

## Decisions

- **Theme State Management**: Use `mode-watcher`. It is already in `package.json` and provides the necessary logic for Svelte apps to handle theme toggling and persistence.
- **Global Layout**: Create `src/routes/+layout.svelte`. This file will host the `<ModeWatcher />` component and any global app shell logic that should persist across route changes.
- **Class-based Dark Mode**: Update Tailwind configuration (if needed) and global CSS to use the `.dark` class for dark mode styles instead of relying solely on `@media (prefers-color-scheme: dark)`.
- **Theme Toggle Component**: Create `src/lib/ThemeToggle.svelte`. It will be a button using raw SVGs for Sun (light) and Moon (dark) icons to avoid unnecessary dependencies.
- **Placement**: Place the `ThemeToggle` at the bottom of the `.sidebar` to keep it accessible but out of the primary content flow.

## Risks / Trade-offs

- **[Risk] Flash of un-themed content (FOUC)** → [Mitigation] `mode-watcher` includes a script to be placed in `src/app.html` (or handled automatically in SvelteKit) that applies the theme class before the page is fully rendered.
- **[Risk] CSS Variable Redundancy** → [Mitigation] Refactor `:global(:root)` and `@media (prefers-color-scheme: dark)` in `+page.svelte` into a more centralized location or ensure they respect the `.dark` class.
