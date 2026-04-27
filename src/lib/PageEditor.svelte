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
  import { tick, untrack } from 'svelte';
  import { wikilinkPlugin, attachPreviewClickHandler } from './wikilink-plugin';
  import {
    app,
    activeVault,
    pageSlugs,
    openPage,
    createPage,
    savePage,
    renamePage,
    loadAttachments,
    extractTitle,
  } from './stores.svelte';

  // ── Props ─────────────────────────────────────────────────────────────────────

  let { 
    slug,
    onSaving = (s: boolean) => {}
  }: { 
    slug: string;
    onSaving?: (s: boolean) => void;
  } = $props();

  import { isInputFocused, isModKey } from './hotkeys';

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

  // Create a stable Carta instance that doesn't re-create on every slug change.
  // The plugins use getters so they stay up to date with the latest props.
  const carta = new Carta({
    sanitizer: false,
    theme: {
      light: 'material-theme-lighter',
      dark: 'material-theme-palenight',
    },
    shikiOptions: {
      themes: ['material-theme-lighter', 'material-theme-palenight'],
    },
    extensions: [
      wikilinkPlugin({
        getPages: () => pageSlugs(),
        onNavigate: (s) => openPage(s),
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
      code({
        theme: {
          light: 'material-theme-lighter',
          dark: 'material-theme-palenight'
        }
      }),
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
            const encoded = encodeURIComponent(filename).replace(/%20/g, '%20');
            return `assets/${encoded}`;
          } catch (e) {
            console.error('Upload failed:', e);
            return null;
          }
        }
      })
    ],
  });

  // ── Effects ──────────────────────────────────────────────────────────────────

  // Load content when slug changes
  $effect(() => {
    // We only want to run this when 'slug' changes, not when app.currentContent updates from auto-save
    const targetSlug = slug; 
    untrack(() => {
      const page = app.pages.find((p) => p.slug === targetSlug);
      if (page) {
        content = app.currentContent ?? '';
      }
      
      // Reset scroll position
      if (editorContainer) {
        const input = editorContainer.querySelector('.carta-input');
        const renderer = editorContainer.querySelector('.carta-renderer');
        if (input) input.scrollTop = 0;
        if (renderer) renderer.scrollTop = 0;
      }
    });
  });

  // Auto-save
  $effect(() => {
    const value = content;
    if (!value && !app.currentContent) return;
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(async () => {
      const targetSlugForSave = slug;
      saving = true;
      try {
        // ── Detect title change for auto-rename ──
        const extractedTitle = extractTitle(value);
        const currentPage = app.pages.find(p => p.slug === slug);
        
        if (extractedTitle && currentPage && extractedTitle !== currentPage.title) {
          // Title changed in markdown, trigger rename
          await renamePage(slug, extractedTitle, value);
          // renamePage already calls loadPages and openPage(newSlug)
        } else {
          const finalContent = await savePage(slug, value);
          // Only update currentContent if the slug hasn't changed in the meantime
          if (slug === targetSlugForSave) {
            app.currentContent = finalContent ?? value;
            // If backend modified the content (e.g. prepended title), update the editor
            if (finalContent && finalContent !== value) {
              content = finalContent;
            }
          }
        }
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

  /**
   * Focuses the editor's textarea with a retry mechanism to ensure the DOM is ready.
   */
  async function focusEditor() {
    await tick();
    const textarea = carta.input?.textarea;
    if (!textarea) return;

    textarea.focus();
    const end = textarea.value.length;
    textarea.setSelectionRange(end, end);
  }

  // Handle 'i' and 'Esc' keys for mode switching
  $effect(() => {
    function handleKeydown(e: KeyboardEvent) {
      // Ignore if modal is open
      if (app.showSearch || app.showExitPrompt) return;

      const modI = isModKey(e, 'i');

      if (isInputFocused()) {
        // If in editor, only handle Escape to switch to preview
        if (e.key === 'Escape' && app.editorTab === 'write') {
          e.preventDefault();
          e.stopPropagation();
          app.editorTab = 'preview';
          carta.input?.textarea?.blur();
        }
        return;
      }

      // Enter edit mode: 'i' or 'Ctrl+I'
      if ((e.key === 'i' || modI) && app.editorTab === 'preview') {
        e.preventDefault();
        app.editorTab = 'write';
        focusEditor();
      }
    }

    // Use capture phase to ensure we get the event before Carta or others consume it
    window.addEventListener('keydown', handleKeydown, true);
    return () => window.removeEventListener('keydown', handleKeydown, true);
  });

  function toggleMode() {
    app.editorTab = app.editorTab === 'write' ? 'preview' : 'write';
    if (app.editorTab === 'write') {
      focusEditor();
    }
  }
</script>

<div class="page-editor" class:write-mode={app.editorTab === 'write'} bind:this={editorContainer}>
  <button 
    class="mode-toggle-btn" 
    onclick={toggleMode}
    title={app.editorTab === 'write' ? 'Switch to Preview (Esc)' : 'Switch to Edit (i)'}
    aria-label="Toggle mode"
  >
    {#if app.editorTab === 'write'}
      <Eye size={16} />
    {:else}
      <Edit3 size={16} />
    {/if}
  </button>

  {#key app.editorTab}
    <MarkdownEditor
      {carta}
      bind:value={content}
      selectedTab={app.editorTab}
      mode="tabs"
      theme={mode.current === 'dark' ? 'dark' : 'light'}
    />
  {/key}
</div>

<style>
  .page-editor {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: var(--bg);
    position: relative;
    overflow: hidden;
    isolation: isolate;
  }

  .page-editor.write-mode {
    background: var(--bg);
  }

  .page-editor.write-mode::before {
    content: '';
    position: absolute;
    inset: 0;
    background-image:
      radial-gradient(circle at 1px 1px, color-mix(in oklab, var(--muted-foreground) 16%, transparent) 1px, transparent 0),
      radial-gradient(circle at 50% 50%, color-mix(in oklab, var(--muted-foreground) 12%, transparent) 1px, transparent 0);
    background-size: 16px 16px, 16px 16px;
    background-position: -1px -1px, 7px 7px;
    opacity: 0.62;
    pointer-events: none;
    z-index: 0;
  }

  .page-editor > * {
    position: relative;
    z-index: 1;
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

  .page-editor :global(.carta-editor) {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border: none;
    border-radius: 0;
    background: transparent;
  }

  .page-editor :global(.carta-wrapper) {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    background: transparent;
  }

  .page-editor :global(.carta-container) {
    height: 100%;
    background: transparent;
  }

  .page-editor :global(.carta-input) {
    height: 100%;
    overflow-y: auto;
    background: transparent;
  }

  .page-editor :global(.carta-renderer) {
    height: 100%;
    overflow-y: auto;
    background: transparent;
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
