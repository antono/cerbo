## Context

Page preview already renders markdown to HTML, but task list items are treated like ordinary list content. The change needs to make GitHub-style task lists feel native in preview: no bullets, clearer checkbox affordance, and direct interaction that updates the markdown source so preview and editor state stay aligned.

This touches preview rendering, DOM event handling, and the page save path. The main constraint is preserving existing layout and line height while changing only the presentation and interaction model for task list items.

## Goals / Non-Goals

**Goals:**
- Render `contains-task-list` items without bullets in page preview.
- Increase checkbox visual size without changing line height or surrounding text flow.
- Allow preview checkboxes to toggle checked state on click.
- Persist checkbox changes back into the markdown source for the page.

**Non-Goals:**
- Reworking general markdown rendering beyond task list items.
- Changing task list syntax support in the editor input model.
- Adding new markdown features unrelated to checkbox lists.

## Decisions

- Use targeted preview styling for task list containers and list markers rather than a broad markdown theme change. This keeps the change isolated and avoids unintended list formatting regressions elsewhere in preview.
- Treat the checkbox as an interactive preview affordance backed by the rendered markdown source. This is preferable to making preview read-only, because the feature explicitly requires state changes to persist.
- Update the markdown source through the same page content save pipeline used for other edits. That keeps persistence, file watching, and downstream indexing behavior consistent.
- Prefer DOM event handling scoped to preview task-list elements over replacing the markdown renderer. That reduces risk and preserves existing preview behavior for links, headings, and other markdown constructs.

## Risks / Trade-offs

- Interactive preview edits can conflict with other concurrent content updates → Re-read or patch the latest page content before writing back.
- Styling checkbox size without altering line height may vary across browsers → Use explicit sizing and alignment rules tied to the task-list checkbox only.
- Direct preview mutation can drift from editor state if the content model is not refreshed after save → Ensure the page content source is reloaded or synchronized after each toggle.
- Scoping behavior to preview only means write-mode markdown remains unchanged until saved → This is intentional to keep the interaction model predictable.
