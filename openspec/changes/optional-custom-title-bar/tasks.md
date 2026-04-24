## 1. Configuration Updates

- [ ] 1.1 Add custom title bar preference toggling in tauri.conf.json (decorations dynamic toggle)
- [ ] 1.2 Create settings store for custom title bar preference (localStorage)

## 2. Title Bar Component

- [ ] 2.1 Create CustomTitleBar.svelte component
- [ ] 2.2 Implement theme-aware CSS (use existing CSS variables)
- [ ] 2.3 Add window controls (close, minimize, maximize) using @tauri-apps/api/window
- [ ] 2.4 Implement draggable title area using startDragging()

## 3. Settings Integration

- [ ] 3.1 Add custom title bar toggle in app settings UI
- [ ] 3.2 Wire up localStorage persistence for preference

## 4. Theme Sync Updates

- [ ] 4.1 Update +layout.svelte to sync theme with custom title bar (not just webview)
- [ ] 4.2 Add effect to update custom title bar when theme changes

## 5. Integration

- [ ] 5.1 Conditionally render custom title bar or use native decorations based on preference
- [ ] 5.2 Test toggle between custom and native decorations
- [ ] 5.3 Test theme changes with custom title bar enabled