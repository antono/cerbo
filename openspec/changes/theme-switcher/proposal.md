## Why

Users need the ability to switch between light and dark themes to optimize readability and comfort in different environments. Providing a manual switcher ensures accessibility and user preference control.

## What Changes

- **Theme Toggle UI**: A single button in the UI (likely in a header or sidebar) that switches between light and dark modes.
- **Iconic Feedback**: The button will display a Sun icon for light mode and a Moon icon for dark mode.
- **Persistence**: The chosen theme will persist across sessions using `mode-watcher`.
- **System Synchronization**: Optional initial sync with system theme preferences.
- **Global Styling**: Implementation of Tailwind dark mode classes across existing components.

## Capabilities

### New Capabilities
- `theme-management`: Logic for theme state, persistence, and application using `mode-watcher`.
- `theme-toggle-component`: A reusable UI component for switching themes with visual feedback.

### Modified Capabilities
- `vault-management`: The main layout will likely need to be updated to include the theme provider and the switcher.

## Impact

- `src/routes/+layout.svelte`: New file to host `<ModeWatcher />` and global layout structure.
- `src/lib/`: New component for the theme toggle.
- `package.json`: Already includes `mode-watcher` and `tailwindcss`.
- `tailwind.config.js`: Ensure `darkMode: 'class'` or similar is configured if using Tailwind 4.
