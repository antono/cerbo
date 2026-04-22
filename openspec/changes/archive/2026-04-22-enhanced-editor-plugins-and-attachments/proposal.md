## Why
The current editor experience is limited to basic markdown, lacking essential productivity features like syntax highlighting, emoji support, and direct file attachments. Furthermore, there is no centralized way to view or manage assets (images, documents) associated with a specific page, hindering the vault's utility as a comprehensive knowledge base.

## What Changes
- **Enhanced Editor**: Integrate Carta plugins for syntax highlighting (`code`), emoji support (`emoji`), heading anchors (`anchor`), and a file upload interface (`attachment`).
- **Attachment Management**: Implement a persistent storage system where files uploaded through the editor are saved into an `assets/` subfolder within the page's directory.
- **Sidebar Integration**: Introduce an "Attachments" section in the right sidebar, allowing users to view, delete, and drag-and-drop existing assets into the editor.
- **Backend & CLI**: Extend the `@core` library with attachment lifecycle management and expose these capabilities through new `@cli` subcommands.
- **Unified API**: Add Tauri commands to handle file uploads, listing page-specific assets, and deleting assets, bridging the frontend to the new core logic.

## Capabilities

### New Capabilities
- `page-attachments`: Requirements for managing file life-cycle within a page folder, including storage layout (`<slug>/assets/`), listing, and deletion. Includes CLI support for these operations.

### Modified Capabilities
- `page-crud`: Update editor requirements to include support for the new plugin set and attachment upload handlers.
- `backlinks`: Update layout requirements as the right sidebar now becomes a multi-section panel (Backlinks + Attachments).

## Impact
- `src/lib/PageEditor.svelte`: Plugin integration and upload logic.
- `src/lib/AttachmentsPanel.svelte`: New component for the sidebar.
- `src/routes/+page.svelte`: Layout adjustments to accommodate the new sidebar section.
- `src-tauri/src/page.rs`: New commands for file operations.
- `package.json`: Addition of `@cartamd/plugin-*` dependencies.
