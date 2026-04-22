<script lang="ts">
  import { Carta, MarkdownEditor } from 'carta-md';
  import 'carta-md/default.css';
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
    isEditing = $bindable(false),
    onSaving = (s: boolean) => {}
  }: { 
    slug: string;
    isEditing?: boolean;
    onSaving?: (s: boolean) => void;
  } = $props();

  // ── State ────────────────────────────────────────────────────────────────────

  let content = $state('');
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let saving = $state(false);
  let previewEl = $state<HTMLElement | null>(null);

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
            isEditing = false;
            openPage(s);
          },
          onCreate: (title) => {
            createPage(title).then((newSlug) => {
              isEditing = true; // Edit newly created page
              openPage(newSlug);
            }).catch((e) => {
              app.error = String(e);
            });
          },
        }),
      ],
    }),
  );

  // ── Effects ──────────────────────────────────────────────────────────────────

  // Load content when slug changes
  $effect(() => {
    const page = app.pages.find((p) => p.slug === slug);
    if (page) {
      content = app.currentContent ?? '';
      // Default to preview when switching pages
      isEditing = false;
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

  // Render preview and attach links
  let renderedHtml = $state('');
  $effect(() => {
    if (!isEditing) {
      carta.render(content).then(html => {
        renderedHtml = html;
      });
    }
  });

  $effect(() => {
    if (previewEl && !isEditing) {
      return attachPreviewClickHandler(previewEl, {
        onNavigate: (s) => openPage(s),
        onCreate: (t) => createPage(t)
      });
    }
  });
</script>

<div class="page-editor">
  <div class="editor-content-wrap">
    {#if isEditing}
      <MarkdownEditor
        {carta}
        bind:value={content}
        mode="single"
      />
    {:else}
      <div 
        bind:this={previewEl}
        class="preview-mode carta-renderer"
      >
        {@html renderedHtml}
      </div>
    {/if}
  </div>
</div>

<style>
  .page-editor {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .editor-content-wrap {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  .preview-mode {
    padding: 2rem;
    max-width: 800px;
    margin: 0 auto;
  }

  /* Make sure links look interactive in preview */
  :global(.preview-mode a.wikilink) {
    color: var(--primary);
    text-decoration: none;
    border-bottom: 1px dashed var(--primary);
  }

  :global(.preview-mode a.wikilink-broken) {
    color: #dc2626;
    border-bottom-color: #dc2626;
  }

  .page-editor :global(.carta-editor) {
    height: 100%;
  }
</style>
