<script lang="ts">
  import { Carta, MarkdownEditor } from 'carta-md';
  import 'carta-md/default.css';
  import { code } from '@cartamd/plugin-code';
  import { emoji } from '@cartamd/plugin-emoji';
  import { anchor } from '@cartamd/plugin-anchor';
  import { attachment } from '@cartamd/plugin-attachment';
  import { invoke } from '@tauri-apps/api/core';
  import { wikilinkPlugin, attachPreviewClickHandler } from './wikilink-plugin';
  import {
    app,
    pageSlugs,
    openPage,
    createPage,
    savePage,
  } from './stores.svelte';

  // ── Props ─────────────────────────────────────────────────────────────────────

  let { 
    slug,
    onSaving = (s: boolean) => {}
  }: { 
    slug: string;
    onSaving?: (s: boolean) => void;
  } = $props();

  // ── State ────────────────────────────────────────────────────────────────────

  let content = $state('');
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let saving = $state(false);
  let editorContainer = $state<HTMLElement | null>(null);

  // Notify parent of saving state
  $effect(() => {
    onSaving(saving);
  });

  // ── Carta instance ──────────────────────────────────────────────────────────

  let carta = $derived(
    new Carta({
      sanitizer: false,
      extensions: [
        wikilinkPlugin({
          getPages: () => pageSlugs(),
          onNavigate: (s) => {
            openPage(s);
          },
          onCreate: (title) => {
            createPage(title).then((newSlug) => {
              openPage(newSlug);
            }).catch((e) => {
              app.error = String(e);
            });
          },
        }),
        code(),
        emoji(),
        anchor(),
        attachment({
          upload: async (file) => {
            try {
              // Since we're in a local Tauri environment, "uploading" 
              // means copying the file to the vault's assets directory.
              // Note: web 'File' API doesn't give us the full path, 
              // but Tauri's attachment plugin can handle file drops if configured.
              // For now, we use the file name and content if possible, 
              // or handle it via a Tauri command if we have the path.
              
              // If we have a real path (e.g. from a drop or picker managed by Tauri),
              // we call attachment_add. But web 'File' objects are buffers.
              // We'll use a specialized command that accepts bytes.
              
              const buffer = await file.arrayBuffer();
              const bytes = new Uint8Array(buffer);
              
              // We need to implement attachment_upload in Tauri to handle bytes
              const filename = await invoke<string>('attachment_upload', {
                vault_id: app.activeVaultId,
                slug: slug,
                filename: file.name,
                data: Array.from(bytes)
              });
              
              // Return the markdown link
              return `assets/${filename}`;
            } catch (e) {
              console.error('Upload failed:', e);
              return null;
            }
          }
        })
      ],
    }),
  );

  // ── Effects ──────────────────────────────────────────────────────────────────

  // Load content when slug changes
  $effect(() => {
    const page = app.pages.find((p) => p.slug === slug);
    if (page) {
      content = app.currentContent ?? '';
    }
  });

  // Auto-save
  $effect(() => {
    const value = content;
    if (!value && !app.currentContent) return;
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(async () => {
      saving = true;
      try {
        await savePage(slug, value);
        app.currentContent = value;
      } catch (e) {
        app.error = String(e);
      } finally {
        saving = false;
      }
    }, 800);
  });

  // Attach link click handlers via event delegation on the container
  $effect(() => {
    if (editorContainer) {
      return attachPreviewClickHandler(editorContainer, {
        onNavigate: (s) => openPage(s),
        onCreate: (t) => createPage(t)
      });
    }
  });
</script>

<div class="page-editor" bind:this={editorContainer}>
  <MarkdownEditor
    {carta}
    bind:value={content}
    mode="tabs"
  />
</div>

<style>
  .page-editor {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: var(--bg);
  }

  .page-editor :global(.carta-editor) {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden; /* Prevent editor container from scrolling */
  }

  .page-editor :global(.carta-wrapper) {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .page-editor :global(.carta-container) {
    height: 100%;
  }

  /* Match editor radius and borders to app shell */
  .page-editor :global(.carta-editor) {
    border: none;
    border-radius: 0;
  }

  /* Make sure links look interactive in preview */
  :global(.carta-renderer a.wikilink) {
    color: var(--primary);
    text-decoration: none;
    border-bottom: 1px dashed var(--primary);
  }

  :global(.carta-renderer a.wikilink-broken) {
    color: #dc2626;
    border-bottom-color: #dc2626;
  }
</style>
