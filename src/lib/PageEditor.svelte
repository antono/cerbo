<script lang="ts">
  import { Carta, MarkdownEditor } from 'carta-md';
  import 'carta-md/default.css';
  import { wikilinkPlugin } from './wikilink-plugin';
  import {
    app,
    pageSlugs,
    openPage,
    createPage,
    savePage,
  } from './stores.svelte';

  // ── Props ─────────────────────────────────────────────────────────────────────

  let { slug }: { slug: string } = $props();

  // ── Carta instance (recreated when vault changes so page list is fresh) ───────

  let carta = $derived(
    new Carta({
      sanitizer: false,
      extensions: [
        wikilinkPlugin({
          getPages: () => pageSlugs(),
          onNavigate: (s) => openPage(s),
          onCreate: (title) => {
            createPage(title).then((newSlug) => openPage(newSlug)).catch((e) => {
              app.error = String(e);
            });
          },
        }),
      ],
    }),
  );

  // ── Content ──────────────────────────────────────────────────────────────────

  let content = $state('');
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let saving = $state(false);

  // Load content when slug changes
  $effect(() => {
    const page = app.pages.find((p) => p.slug === slug);
    if (page) {
      content = app.currentContent ?? '';
    }
  });

  // Auto-save: watch content changes and debounce writes
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
</script>

<div class="page-editor">
  {#if saving}
    <div class="save-indicator">Saving…</div>
  {/if}
  <MarkdownEditor
    {carta}
    bind:value={content}
    mode="split"
    scroll="sync"
  />
</div>

<style>
  .page-editor {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .save-indicator {
    position: absolute;
    top: 0.5rem;
    right: 1rem;
    font-size: 0.75rem;
    color: var(--muted-foreground, #888);
    z-index: 10;
    pointer-events: none;
  }

  .page-editor :global(.carta-editor) {
    flex: 1;
    min-height: 0;
    height: 100%;
  }
</style>
