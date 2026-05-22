# Design: UI Metadata – Custom Slug and Virtual Path

## Context

The new page dialog currently captures only the page name. Users need the ability to customize the URL slug and organize pages within virtual mount paths. This design adds two new form fields with intelligent defaults while preserving user intent when manual edits are made.

Tech stack: SvelteKit frontend, Tauri for desktop, Rust backend.

## Goals / Non-Goals

**Goals:**
- Provide smart slug defaults that reduce user input while allowing customization
- Enable path-based organization with autocomplete for better UX
- Preserve user overrides—once a slug is manually edited, stop suggesting changes
- Keep implementation simple: text input fields with client-side validation and autocomplete

**Non-Goals:**
- Visual path picker or tree interface (text input with autocomplete only)
- Backend slug normalization or conflict resolution (validate on submit)
- Historical tracking of slug changes or undo functionality

## Decisions

### 1. Dialog State Management
**Decision**: Add a `slugAutoUpdateEnabled` boolean flag to the new-page dialog state.

**Rationale**: Track whether slug should auto-update as page name changes. Once user edits the slug field manually (first edit), set this flag to `false` and never auto-update during that dialog session.

**Alternatives Considered**:
- Store slug "generation time" and compare to detect edits → more complex, same result
- Re-generate slug every keystroke unless unchanged → can't distinguish user edit from auto-update

### 2. Slug Generation Algorithm
**Decision**: Transform page name to slug using: lowercase, trim whitespace, replace spaces with hyphens, remove non-alphanumeric/hyphen/underscore characters.

Example: "My New Page" → "my-new-page"

**Rationale**: Standard URL-friendly format, consistent with REST conventions. Generate only when page name changes and `slugAutoUpdateEnabled` is true.

**Alternatives Considered**:
- Use transliteration library for Unicode → adds dependency, overkill for simple cases
- Always validate slug format server-side → good practice, but client should prevent invalid input

### 3. Virtual Path Autocomplete
**Decision**: On input to the virtual path field, fetch all existing paths from the vault and filter/suggest matching segments.

Autocomplete list shows paths that start with user input (case-insensitive prefix match).

**Rationale**: Paths are hierarchical (`docs/guides/tips`). Prefix matching helps users navigate the namespace. Autocomplete is loaded from existing vault state, not a separate backend call.

**Alternatives Considered**:
- Full-text search across path segments → harder to predict what user wants
- Hardcoded path templates → inflexible, doesn't reflect actual vault structure

### 4. Form Structure
**Decision**: Extend the new-page dialog form with two new fields in this order:
1. Page Name (existing)
2. Slug (new)
3. Virtual Path (new)

**Rationale**: Natural progression—name → identifier → location. Slug depends on name, path is independent. Client-side validation before submit.

## Risks / Trade-offs

**[Risk] User forgets slug is no longer auto-updating**
→ *Mitigation*: Show subtle visual indicator (e.g., icon or disabled state on slug field) once user makes first edit

**[Risk] Autocomplete list could be very long if vault has deep paths**
→ *Mitigation*: Limit autocomplete results to top N matches, make list scrollable

**[Risk] Slug collisions not detected until form submit**
→ *Mitigation*: Accept this—validation happens server-side. Show error message if slug already exists after submit

**[Trade-off] Autocomplete requires loading existing paths into client state**
→ Accept for now. Paths are typically small dataset. If vault scales, optimize by lazy-loading or pagination.

## Migration Plan

1. Update new-page dialog component to add slug and virtualPath fields
2. Add slug generation utility function
3. Fetch existing paths from vault context and wire autocomplete
4. Update form submission to pass slug and virtualPath to page creation API
5. Add basic client-side validation (no empty fields, slug format)
6. Ship and monitor for slug collision errors

No backend changes required; API already supports slug and virtualPath parameters.

## Open Questions

- Should slug validation allow other characters (dots, underscores beyond hyphens)?
- Should autocomplete offer suggestions for partial path segments (e.g., "docs/g" → suggest "guides")?
- Visual feedback for "slug no longer auto-updating"—icon, disabled state, or tooltip?
