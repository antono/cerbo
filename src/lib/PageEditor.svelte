<script lang="ts">
  import { Carta, MarkdownEditor } from 'carta-md';
  import 'carta-md/default.css';
  import { code } from '@cartamd/plugin-code';
  import '@cartamd/plugin-code/default.css';
  import { emoji } from '@cartamd/plugin-emoji';
  import { anchor } from '@cartamd/plugin-anchor';
  import { attachment } from '@cartamd/plugin-attachment';
  import { invoke } from '@tauri-apps/api/core';
  import { mode } from 'mode-watcher';
  import { Eye, Edit3 } from 'lucide-svelte';
  import { wikilinkPlugin, attachPreviewClickHandler } from './wikilink-plugin';
  import {
    app,
    activeVault,
    pageSlugs,
    openPage,
    createPage,
    savePage,
    loadAttachments,
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
  let selectedTab = $state<'write' | 'preview'>('write');

  // Notify parent of saving state
  $effect(() => {
    onSaving(saving);
  });

  // ── Carta instance ──────────────────────────────────────────────────────────

  let carta = $derived(
    new Carta({
      sanitizer: false,
      theme: mode.current === 'dark' ? 'github-dark' : 'github-light',
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
          getVaultPath: () => activeVault()?.path,
          getSlug: () => slug,
          onOpenAsset: (filename) => {
            invoke('attachment_open', {
              vaultId: app.activeVaultId,
              slug: slug,
              filename: filename
            });
          }
        }),
        code(),
        emoji(),
        anchor(),
        attachment({
          upload: async (file) => {
            try {
              const buffer = await file.arrayBuffer();
              const bytes = new Uint8Array(buffer);
              
              const filename = await invoke<string>('attachment_upload', {
                vaultId: app.activeVaultId,
                slug: slug,
                filename: file.name,
                data: Array.from(bytes)
              });
              
              await loadAttachments(slug);
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
        onCreate: (t) => createPage(t),
        onOpenAsset: (f) => invoke('attachment_open', {
          vaultId: app.activeVaultId,
          slug: slug,
          filename: f
        })
      });
    }
  });

  function toggleMode() {
    selectedTab = selectedTab === 'write' ? 'preview' : 'write';
  }
</script>

<div class="page-editor" class:edit-mode={selectedTab === 'write'} bind:this={editorContainer}>
  <button 
    class="mode-toggle-btn" 
    onclick={toggleMode}
    title={selectedTab === 'write' ? 'Switch to Preview' : 'Switch to Edit'}
    aria-label="Toggle mode"
  >
    {#if selectedTab === 'write'}
      <Eye size={16} />
    {:else}
      <Edit3 size={16} />
    {/if}
  </button>

  <MarkdownEditor
    {carta}
    bind:value={content}
    selectedTab={selectedTab}
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
    transition: background-color 0.2s;
    position: relative;
  }

  .mode-toggle-btn {
    position: absolute;
    top: 0;
    left: 0.5rem;
    height: var(--header-height);
    width: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    background: transparent;
    border: none;
    cursor: pointer;
    color: var(--muted-foreground);
    transition: all 0.15s;
    border-radius: var(--radius);
  }

  .mode-toggle-btn:hover {
    background: var(--accent-hover);
    color: var(--fg);
  }

  .page-editor.edit-mode {
    --bg: #fff1f1;
  }

  :global(.dark) .page-editor.edit-mode {
    --bg: #1a0f0f;
  }

  .page-editor :global(.carta-editor) {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .page-editor :global(.carta-wrapper) {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .page-editor :global(.carta-container) {
    height: 100%;
  }

  .page-editor :global(.carta-editor) {
    border: none;
    border-radius: 0;
  }

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
