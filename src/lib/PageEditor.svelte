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
  }

  .page-editor :global(.carta-editor) {
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .page-editor :global(.carta-wrapper) {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  .page-editor :global(.carta-container) {
    height: 100%;
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
