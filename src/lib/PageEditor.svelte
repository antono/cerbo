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
  import { onMount, tick, untrack } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import ExternalChangeDialog from './ExternalChangeDialog.svelte';
  import { wikilinkPlugin, attachPreviewClickHandler } from './wikilink-plugin';
  import { getCursorPositionFromOffset, restoreCursorPosition as resolveCursorPosition } from './cursor-position';
  import { decideExternalPageChange, logPageContentDiff, pageChangeKey, pageMdPathToSlug, shouldIgnoreUnchangedPageChange, shouldSkipExternalPageChange } from './page-sync';
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
  let baselineContent = $state('');
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let saving = $state(false);
  let editorContainer = $state<HTMLElement | null>(null);
  let showConflictPrompt = $state(false);
  let lastConflictKey = '';
  let suppressedConflictKey: string | null = null;

  // Notify parent of saving state
  $effect(() => {
    onSaving(saving);
  });

  let isDirty = $derived(content !== baselineContent);

  let loadedSlug = $state<string | null>(null);

  $effect(() => {
    const currentSlug = app.currentSlug;
    if (!currentSlug || currentSlug === loadedSlug) return;
    loadedSlug = currentSlug;

    untrack(() => {
      const currentContent = app.currentContent ?? '';
      baselineContent = currentContent;
      content = currentContent;
      showConflictPrompt = false;
    });
  });

  onMount(() => {
    let unlisten: (() => void) | null = null;
    void listen<{ vaultId: string; path: string }>('page-file-changed', (event) => {
      const vault = activeVault();
      const currentSlug = app.currentSlug;
      if (!vault || event.payload.vaultId !== vault.id || !currentSlug) return;

      const conflictKey = pageChangeKey(event.payload.vaultId, event.payload.path);
      if (shouldSkipExternalPageChange(conflictKey, suppressedConflictKey)) {
        suppressedConflictKey = null;
        return;
      }
      if (showConflictPrompt && conflictKey === lastConflictKey) return;
      lastConflictKey = conflictKey;

      const changedSlug = pageMdPathToSlug(vault.path, event.payload.path);
      const action = decideExternalPageChange({
        currentSlug,
        changedSlug,
        editorTab: app.editorTab,
        dirty: isDirty,
      });

      const readCurrentDiskContent = async () => {
        const nextContent = await invoke<string>('page_read', {
          vaultId: vault.id,
          slug: currentSlug,
        });
        return nextContent;
      };

      if (action === 'ignore') return;
      void (async () => {
        const nextContent = await readCurrentDiskContent();
        if (shouldIgnoreUnchangedPageChange(content, nextContent)) {
          return;
        }

        logPageContentDiff(`[page-change-diff] ${currentSlug}`, content, nextContent);

        if (action === 'reload') {
          app.currentContent = nextContent;
          baselineContent = nextContent;
          content = nextContent;
          return;
        }

        showConflictPrompt = true;
      })();
    }).then((stop) => {
      unlisten = stop;
    });

    return () => unlisten?.();
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
          suppressNextOwnPageChange();
          const finalContent = await savePage(slug, value);
          // Only update currentContent if the slug hasn't changed in the meantime
          if (slug === targetSlugForSave) {
            app.currentContent = finalContent ?? value;
            baselineContent = finalContent ?? value;
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
  }

  async function saveCursorPosition() {
    const textarea = carta.input?.textarea;
    if (!textarea || !app.activeVaultId || !app.currentSlug) return;
    const selectionStart = textarea.selectionStart ?? 0;
    const cursor = getCursorPositionFromOffset(textarea.value, selectionStart);

    await invoke('cursor_position_save', {
      vaultId: app.activeVaultId,
      slug: app.currentSlug,
      line: cursor.line,
      column: cursor.column,
    });
  }

  async function restoreCursorPosition() {
    if (!app.activeVaultId || !app.currentSlug) return;

    const saved = await invoke<{ line: number; column: number } | null>('cursor_position_load', {
      vaultId: app.activeVaultId,
      slug: app.currentSlug,
    });

    const textarea = carta.input?.textarea;
    if (!textarea) return;

    const { offset } = resolveCursorPosition(textarea.value, saved);

    await focusEditor();
    textarea.setSelectionRange(offset, offset);
    await tick();
    textarea.scrollIntoView({ block: 'center' });
  }

  $effect(() => {
    if (app.editorTab !== 'write') return;
    void restoreCursorPosition();
  });

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
          void saveCursorPosition();
          app.editorTab = 'preview';
          carta.input?.textarea?.blur();
        }
        return;
      }

      // Enter edit mode: 'i' or 'Ctrl+I'
      if ((e.key === 'i' || modI) && app.editorTab === 'preview') {
        e.preventDefault();
        app.editorTab = 'write';
      }
    }

    // Use capture phase to ensure we get the event before Carta or others consume it
    window.addEventListener('keydown', handleKeydown, true);
    return () => window.removeEventListener('keydown', handleKeydown, true);
  });

  function toggleMode() {
    app.editorTab = app.editorTab === 'write' ? 'preview' : 'write';
    if (app.editorTab === 'preview') {
      void saveCursorPosition();
    }
  }

  async function loadCurrentPageFromDisk() {
    const vault = activeVault();
    if (!vault || !app.currentSlug) return;

    const nextContent = await invoke<string>('page_read', {
      vaultId: vault.id,
      slug: app.currentSlug,
    });
    logPageContentDiff(`[page-change-diff] ${app.currentSlug}`, content, nextContent);
    app.currentContent = nextContent;
    baselineContent = nextContent;
    content = nextContent;
    showConflictPrompt = false;
    lastConflictKey = '';
  }

  function suppressNextOwnPageChange() {
    const vault = activeVault();
    if (!vault) return;
    suppressedConflictKey = pageChangeKey(vault.id, `${vault.path}/${slug}/page.md`);
  }

  async function overwriteCurrentPage() {
    suppressNextOwnPageChange();
    showConflictPrompt = false;
    lastConflictKey = '';
    await savePage(slug, content);
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

  {#if showConflictPrompt}
    <ExternalChangeDialog
      onLoad={loadCurrentPageFromDisk}
      onOverwrite={overwriteCurrentPage}
    />
  {/if}
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
