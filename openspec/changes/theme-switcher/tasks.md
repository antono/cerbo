## 1. Infrastructure & Layout

- [ ] 1.1 Create `src/routes/+layout.svelte` to host the global app structure and `<ModeWatcher />`.
- [ ] 1.2 Verify `src/app.html` is compatible with `mode-watcher` (ensure `%sveltekit.head%` is present).

## 2. Theme Toggle Component

- [ ] 2.1 Create `src/lib/ThemeToggle.svelte` with inline SVG icons for Light (Sun) and Dark (Moon) modes.
- [ ] 2.2 Implement the toggle logic using `mode-watcher`'s `setMode` and `resetMode` (for system sync).

## 3. Integration & Styling

- [ ] 3.1 Integrate `ThemeToggle` into the sidebar of `src/routes/+page.svelte`.
- [ ] 3.2 Update global CSS variables in `src/routes/+page.svelte` to use `.dark` class selectors instead of (or in addition to) `@media (prefers-color-scheme: dark)`.
- [ ] 3.3 Audit existing components for any hardcoded colors and replace with theme-aware CSS variables or Tailwind utility classes.
- [ ] 3.4 Verify that theme preference persists after a page reload and application restart (Tauri).
