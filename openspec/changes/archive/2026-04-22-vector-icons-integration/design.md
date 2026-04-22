## Context
The app currently uses a fragmented set of icons: hardcoded SVG paths in `ThemeToggle.svelte`, Unicode characters/emojis for vault icons, and no consistent icon system for UI actions. This lacks visual cohesion and complicates maintenance of the light/dark theme support.

## Goals / Non-Goals

**Goals:**
- Standardize on `lucide-svelte` as the primary vector icon system.
- Ensure all UI chrome icons support automatic inversion (light/dark theme).
- Achieve a consistent "outline" aesthetic with 2px stroke and 24px base size.
- Simplify components by removing hardcoded SVG paths.

**Non-Goals:**
- Replacing user-provided icons or emojis within page content.
- Custom SVG generation or hosting.

## Decisions
- **Library Choice: Lucide Svelte**: Selected for its minimal aesthetic, community-driven nature, and excellent Svelte 5 (Runes) compatibility. Compared to FontAwesome (too heavy) and Iconify (inconsistent styles), Lucide provides the most polished "out-of-the-box" experience for a minimal wiki.
- **Theming: currentColor**: All icons will use the `currentColor` stroke attribute to inherit the `--fg` or `--muted-foreground` CSS variables defined in `app.css`.

## Risks / Trade-offs
- **[Risk] Bundle Size** → **Mitigation**: Use tree-shakable imports (`import { IconName } from 'lucide-svelte'`) to ensure only used icons are bundled.
- **[Risk] Style Mismatch** → **Mitigation**: Standardize on a specific `stroke-width` (default 2px) and `size` (16px or 18px for sidebar/buttons) across the entire app.
