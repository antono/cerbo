## 1. Infrastructure & Layout

- [x] 1.1 Create `src/routes/+layout.svelte` to host the global app structure and `<ModeWatcher />`.
- [x] 1.2 Verify `src/app.html` is compatible with `mode-watcher` (ensure `%sveltekit.head%` is present).

## 2. Theme Toggle Component

- [x] 2.1 Create `src/lib/ThemeToggle.svelte` with inline SVG icons for Light (Sun) and Dark (Moon) modes.
- [x] 2.2 Implement the toggle logic using `mode-watcher`'s `setMode` and `resetMode` (for system sync).

## 3. Integration & Styling

- [x] 3.1 Integrate `ThemeToggle` into the sidebar of `src/routes/+page.svelte`.
- [x] 3.2 Update global CSS variables in `src/routes/+page.svelte` to use `.dark` class selectors instead of (or in addition to) `@media (prefers-color-scheme: dark)`.
- [x] 3.3 Audit existing components for any hardcoded colors and replace with theme-aware CSS variables or Tailwind utility classes.
- [x] 3.4 Verify that theme preference persists after a page reload and application restart (Tauri).
