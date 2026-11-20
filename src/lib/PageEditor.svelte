<script lang="ts">
  import { Carta, MarkdownEditor } from 'carta-md';
  import 'carta-md/default.css';
  import { code } from '@cartamd/plugin-code';
  import '@cartamd/plugin-code/default.css';
  import { emoji } from '@cartamd/plugin-emoji';
  import { anchor } from '@cartamd/plugin-anchor';
  import { invoke } from '@tauri-apps/api/core';
  import { mode } from 'mode-watcher';
  import { onMount, tick, untrack } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import ExternalChangeDialog from './ExternalChangeDialog.svelte';
  import PlainMarkdownInput from './PlainMarkdownInput.svelte';
  import {
    caretTopInText,
    INPUT_MODE_RECHECK_MS,
    OVERLAY_HIGHLIGHT_DELAY_MS,
    captureInputView,
    gatePreviewRendering,
    nextPlainInput,
    restoreInputView,
    textTopAtOffset,
    usePlainInput,
  } from './editor-performance';
  import {
    wikilinkPlugin,
    attachPreviewClickHandler,
    attachTaskListClickHandler,
    previewTaskListPlugin,
  } from './wikilink-plugin';
  import { mentionPlugin } from './mention-plugin';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import {
    cursorKey,
    getCursorPositionFromOffset,
    restoreCursorPosition as resolveCursorPosition,
    scrollTopShowingCaret,
    type SavedCursorPosition,
  } from './cursor-position';
  import { focusTextareaAtOffset } from './textarea-focus';
  import { createJumpHold, currentHeadingIndex, headingsFor, pairRenderedHeadings, type RenderedHeading } from './outline';
  import { setOutlineBridge } from './outline-bridge';
  import { attachEditorAttachments, type DropIndicator } from './editor-attachments';
  import { attachmentFileName } from './attachment-drop';
  import { buildPageDiffFile, decideExternalPageChange, loadPageDiffFile, logPageContentDiff, pageChangeKey, pageMdPathToSlug, shouldIgnoreUnchangedPageChange, shouldSkipExternalPageChange } from './page-sync';
  import {
    app,
    activeVault,
    pageTitles,
    openPage,
    createPage,
    savePage,
    updatePageTitle,
    loadAttachments,
    loadVaultObjects,
    extractTitle,
    onEditorLeave,
    onEditorModeToggle,
  } from './stores.svelte';
  import { toggleTaskListItemAtIndex } from './task-list';
  import { createAutosave } from './autosave';

  // Edits are written this long after the last one.
  const AUTOSAVE_DELAY_MS = 800;
  // How long a window close waits for the last save before the window goes.
  const CLOSE_SAVE_TIMEOUT_MS = 3000;

  // ── Props ─────────────────────────────────────────────────────────────────────

  let {
    uuid,
    onSaving = (s: boolean) => {}
  }: {
    uuid: string;
    onSaving?: (s: boolean) => void;
  } = $props();

  import { isInputFocused, isModKey } from './hotkeys';

  // ── State ────────────────────────────────────────────────────────────────────

  let content = $state('');
  let baselineContent = $state('');
  let saving = $state(false);
  // Saves still running; `saving` stays set until the last one settles.
  let activeSaves = 0;
  let editorContainer = $state<HTMLElement | null>(null);
  let showConflictPrompt = $state(false);
  let previewDiffFile = $state<ReturnType<typeof buildPageDiffFile> | null>(null);
  let lastConflictKey = '';
  let suppressedConflictKey: string | null = null;
  // Set while something is dragged over the editor: where the reference would go.
  let dropIndicator = $state<DropIndicator | null>(null);
  // A reference to a new Attachment was inserted; reload the panel after the next save.
  let attachmentsChanged = false;

  // Notify parent of saving state
  $effect(() => {
    onSaving(saving);
  });

  let isDirty = $derived(content !== baselineContent);

  // The toolbar shows a dot after the title while edits are unsaved or a save
  // runs (spec `desktop-window-toolbar`, "Save state in the toolbar").
  $effect(() => {
    app.pageUnsaved = isDirty || saving;
  });
  $effect(() => () => {
    app.pageUnsaved = false;
  });

  let loadedUuid = $state<string | null>(null);
  // Vault of `loadedUuid`, recorded when the page loads: on a vault switch the
  // position of the page being left belongs to the previous vault.
  let loadedVaultId: string | null = null;

  $effect(() => {
    const currentUuid = app.currentUuid;
    const externalContent = app.currentContent ?? '';

    if (!currentUuid) return;

    untrack(() => {
      if (currentUuid !== loadedUuid) {
        // The textarea still holds the page being left: store where editing it
        // ended, and write edits whose save was still waiting. A vault switch
        // flushed them through the leave hook, before the vault changed.
        void persistCursor(captureCursor());
        if (app.activeVaultId === loadedVaultId) void autosave.flush();
        loadedVaultId = app.activeVaultId;
        loadedUuid = currentUuid;
        baselineContent = externalContent;
        content = externalContent;
        showConflictPrompt = false;
      } else if (content === baselineContent && externalContent !== baselineContent) {
        // Same page, content updated externally (e.g. rename changed the H1), no local edits
        baselineContent = externalContent;
        content = externalContent;
      }
    });
  });

  onMount(() => {
    let unlisten: (() => void) | null = null;
    void listen<{ vaultId: string; path: string }>('page-file-changed', (event) => {
      const vault = activeVault();
      const currentUuid = app.currentUuid;
      if (!vault || event.payload.vaultId !== vault.id || !currentUuid) return;

      const conflictKey = pageChangeKey(event.payload.vaultId, event.payload.path);
      if (shouldSkipExternalPageChange(conflictKey, suppressedConflictKey)) {
        suppressedConflictKey = null;
        return;
      }
      if (showConflictPrompt && conflictKey === lastConflictKey) return;
      lastConflictKey = conflictKey;

      const changedSlug = pageMdPathToSlug(vault.path, event.payload.path);
      const action = decideExternalPageChange({
        currentSlug: currentUuid,
        changedSlug,
        editorTab: app.editorTab,
        dirty: isDirty,
      });

      const readCurrentDiskContent = async () => {
        const nextContent = await invoke<string>('page_read', { uuid: currentUuid });
        return nextContent;
      };

      if (action === 'ignore') return;
      void (async () => {
        const nextContent = await readCurrentDiskContent();
        if (shouldIgnoreUnchangedPageChange(content, nextContent)) {
          return;
        }

        logPageContentDiff(`[page-change-diff] ${currentUuid}`, content, nextContent);

        if (action === 'reload') {
          app.currentContent = nextContent;
          baselineContent = nextContent;
          content = nextContent;
          return;
        }

        showConflictPrompt = true;
        previewDiffFile = null;
      })();
    }).then((stop) => {
      unlisten = stop;
    });

    return () => unlisten?.();
  });

  // ── Carta instance ──────────────────────────────────────────────────────────

  // Create a stable Carta instance that doesn't re-create on every uuid change.
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
      // First, so `[Berlin]{schema:Place}` becomes a mention before link handling.
      mentionPlugin({ getPrefixes: () => app.prefixMap }),
      wikilinkPlugin({
        getPages: () => pageTitles(),
        getVaultObjects: () => app.vaultObjects,
        getPagePaths: () => app.pages,
        onNavigate: (title) => {
          const page = app.pages.find(p => p.title === title);
          if (page) openPage(page.uuid);
        },
        onCreate: (title) => {
          createPage(title).then((newUuid) => {
            openPage(newUuid);
          }).catch((e) => {
            app.error = String(e);
          });
        },
        getVaultPath: () => activeVault()?.path,
        getSlug: () => uuid,
        onOpenAsset: (filename) => {
          invoke('attachment_open', {
            vaultId: app.activeVaultId,
            uuid,
            filename
          });
        }
      }),
      previewTaskListPlugin(),
      code({
        theme: {
          light: 'material-theme-lighter',
          dark: 'material-theme-palenight'
        }
      }),
      emoji(),
      anchor(),
      // Drop and paste are handled by `editor-attachments.ts`, not Carta's
      // attachment plugin, so both share one pipeline.
    ],
  });

  // The write tab still mounts Carta's hidden Renderer; keep it from parsing
  // the page on every keystroke. Switching to preview remounts the editor
  // ({#key} below) and renders the current text once.
  gatePreviewRendering(carta, () => app.editorTab === 'preview');

  // Large pages are written in a plain textarea (no shiki overlay). Decided
  // when a page is loaded or the tab switches, and re-checked once typing
  // pauses (see the effect below), never on the keystroke path.
  let plainInput = $derived.by(() => {
    app.editorTab;
    loadedUuid;
    return untrack(() => usePlainInput(content.length));
  });
  // Preview always uses MarkdownEditor: the input mode must not remount it,
  // or the preview click handlers stay attached to the old renderer.
  let editorKey = $derived(app.editorTab === 'write' ? `write:${plainInput}` : 'preview');

  /** Swaps the write-mode input, keeping caret, selection, focus and scroll. */
  async function switchInputMode(next: boolean) {
    const textarea = carta.input?.textarea;
    const view = textarea ? captureInputView(textarea) : null;
    plainInput = next;
    await tick();
    const nextTextarea = carta.input?.textarea;
    if (!view || !nextTextarea || nextTextarea === textarea) return;
    restoreInputView(nextTextarea, view);
    // Carta sizes its textarea after mount; scroll again once it has.
    requestAnimationFrame(() => restoreInputView(nextTextarea, view));
  }

  // ── Effects ──────────────────────────────────────────────────────────────────

  // Load content when uuid changes
  $effect(() => {
    // We only want to run this when 'uuid' changes, not when app.currentContent updates from auto-save
    const targetUuid = uuid;
    untrack(() => {
      const page = app.pages.find((p) => p.uuid === targetUuid);
      if (page) {
        content = app.currentContent ?? '';
      }

      // The preview starts at the top; the write view's scroll (Carta's input
      // or the plain textarea of large pages) is restored with the caret (see
      // `restoreCursor`).
      const renderer = editorContainer?.querySelector('.carta-renderer');
      if (renderer) renderer.scrollTop = 0;
    });
  });

  // A page that crosses the size threshold while open (paste, typing, an
  // external reload) switches input mode once changes pause for a second.
  $effect(() => {
    const length = content.length;
    if (app.editorTab !== 'write') return;
    const timer = setTimeout(() => {
      const next = nextPlainInput(plainInput, length);
      if (next !== plainInput) void switchInputMode(next);
    }, INPUT_MODE_RECHECK_MS);
    return () => clearTimeout(timer);
  });

  // Auto-save

  /** Write `value` to page `target`; when its H1 changed, update the page title too. */
  async function persistPage(target: string, value: string): Promise<void> {
    activeSaves += 1;
    saving = true;
    try {
      // Not for a page being left: the editor already shows another one.
      if (target === app.currentUuid) suppressNextOwnPageChange();
      const finalContent = await savePage(target, value);
      // The save failed and the error is shown: the edits stay unsaved.
      if (finalContent === undefined) return;
      // Only touch the editor when it still shows the page that was saved.
      if (target === loadedUuid) {
        app.currentContent = finalContent;
        baselineContent = finalContent;
        // If backend modified the content (e.g. prepended title), update the editor
        if (finalContent !== value) content = finalContent;
      }
      // The H1 changed: update the title after the write, so body edits made
      // with it are already on disk. A page that was left is not reopened.
      const extractedTitle = extractTitle(value);
      const savedPage = app.pages.find((p) => p.uuid === target);
      if (extractedTitle && savedPage && extractedTitle !== savedPage.title) {
        await updatePageTitle(target, extractedTitle, { reopen: target === loadedUuid });
      }
      // The panel lists the Attachments page.md links to, so it follows the save.
      if (attachmentsChanged && target === loadedUuid) {
        attachmentsChanged = false;
        await loadAttachments(target);
      }
    } catch (e) {
      app.error = String(e);
    } finally {
      activeSaves -= 1;
      saving = activeSaves > 0;
    }
  }

  const autosave = createAutosave<string>(AUTOSAVE_DELAY_MS, persistPage);

  $effect(() => {
    const value = content;
    untrack(() => {
      if (!loadedUuid) return;
      if (!value && !app.currentContent) return;
      autosave.schedule(loadedUuid, value, value !== baselineContent);
    });
  });

  // Attach link click handlers via event delegation on the container
  $effect(() => {
    if (app.editorTab !== 'preview' || !editorContainer) return;
    {
      const previewSurface = editorContainer.querySelector('.carta-renderer') as HTMLElement | null;
      console.debug('[page-editor] attach preview handler', {
        uuid,
        container: editorContainer,
        previewSurface,
      });
      if (!previewSurface) return;

      return attachPreviewClickHandler(previewSurface, {
        onNavigate: (title) => {
          const page = app.pages.find(p => p.title === title);
          if (page) openPage(page.uuid);
        },
        onCreate: (t) => createPage(t),
        onOpenAsset: (f) => invoke('attachment_open', {
          vaultId: app.activeVaultId,
          uuid,
          filename: f
        }),
        onResolveCerboLink: async (linkUuid) => {
          const page = app.pages.find(p => p.uuid === linkUuid);
          const attachment = app.vaultObjects.find(o => o.uuid === linkUuid && o.objectType === 'Attachment');
          if (page) {
            await openPage(page.uuid);
          } else if (attachment) {
            await invoke('attachment_open', { uuid: linkUuid, filename: attachmentFileName(attachment.title) });
          } else {
            app.error = `Page not found: ${linkUuid}`;
          }
        }
      });
    }
  });

  // Drop and paste files or fragments into the editor (spec `editor-attachment-drop`).
  $effect(() => {
    const container = editorContainer;
    if (!container) return;
    return attachEditorAttachments({
      container,
      getTextarea: () => carta.input?.textarea,
      getPageKey: () => (app.activeVaultId && app.currentUuid ? `${app.activeVaultId}/${app.currentUuid}` : null),
      isActive: () => app.editorTab === 'write' && !!app.currentUuid,
      addPath: (srcPath) => invoke<string>('attachment_add', { uuid, srcPath }),
      addBytes: (name, bytes) =>
        invoke<string>('attachment_upload', bytes, { headers: { 'x-cerbo-filename': encodeURIComponent(name) } }),
      readClipboardPaths: () => invoke<string[]>('clipboard_file_paths'),
      addUrl: (url, fallbackName) => invoke<{ uuid: string; name: string }>('attachment_fetch', { url, fallbackName }),
      onInserted: () => {
        attachmentsChanged = true;
        // Lets the preview resolve the new image and the link picker offer it.
        void loadVaultObjects();
      },
      onError: (message) => {
        app.error = message;
      },
      onIndicator: (indicator) => {
        dropIndicator = indicator;
      },
    });
  });

  $effect(() => {
    if (app.editorTab !== 'preview' || !editorContainer) return;
    {
      const previewSurface = editorContainer.querySelector('.carta-renderer') as HTMLElement | null;
      console.debug('[page-editor] attach task-list handler', {
        uuid,
        hasContainer: !!editorContainer,
        previewSurface,
      });
      if (!previewSurface) return;

      return attachTaskListClickHandler(previewSurface, {
        onToggleTask: (index, checked) => {
          console.debug('[page-editor] toggle request', {
            uuid,
            index,
            checked,
            contentPreview: content.slice(0, 120),
          });
          const updated = toggleTaskListItemAtIndex(content, index, checked);
          if (updated === null) return;
          console.debug('[page-editor] toggle applied', {
            uuid,
            index,
            checked,
            before: content,
            after: updated,
          });
          autosave.cancel();
          content = updated;
          app.currentContent = updated;
          void savePage(uuid, updated).then((finalContent) => {
            if (finalContent && finalContent !== updated) {
              console.debug('[page-editor] toggle save normalized content', {
                uuid,
                index,
                checked,
                finalContent,
              });
              content = finalContent;
              app.currentContent = finalContent;
            }
          }).catch((e) => {
            console.error('[page-editor] toggle save failed', e);
            app.error = String(e);
          });
        },
      });
    }
  });

  // ── Caret and scroll (spec `editor-position-restore`) ───────────────────────

  interface CursorSnapshot {
    vaultId: string;
    uuid: string;
    position: Required<SavedCursorPosition>;
  }

  // Positions saved in this window, so entering write mode right after leaving
  // it does not race the save on its way to `cursor.toml`.
  const cursorCache = new Map<string, SavedCursorPosition>();
  // The page whose position has been restored into the textarea. Until then
  // the selection is not the user's, so nothing is saved. Also exposed as
  // `data-caret-restored` for the e2e tests.
  let restoredUuid = $state<string | null>(null);
  let restoreToken = 0;
  let cursorSaveTimer: ReturnType<typeof setTimeout> | null = null;

  function editorScroller(): HTMLElement | null {
    // Large pages are written in a plain textarea that scrolls itself.
    return (
      editorContainer?.querySelector<HTMLElement>('.carta-plain-input') ??
      editorContainer?.querySelector<HTMLElement>('.carta-input') ??
      null
    );
  }

  /** Where editing the loaded page stands now; null outside a write session. */
  function captureCursor(): CursorSnapshot | null {
    const textarea = carta.input?.textarea;
    if (app.editorTab !== 'write' || !textarea || !loadedVaultId || !loadedUuid || restoredUuid !== loadedUuid) {
      return null;
    }
    const { line, column } = getCursorPositionFromOffset(textarea.value, textarea.selectionStart ?? 0);
    const scrollTop = editorScroller()?.scrollTop ?? 0;
    return { vaultId: loadedVaultId, uuid: loadedUuid, position: { line, column, scrollTop } };
  }

  async function persistCursor(snapshot: CursorSnapshot | null) {
    if (cursorSaveTimer) clearTimeout(cursorSaveTimer);
    cursorSaveTimer = null;
    if (!snapshot) return;
    const { vaultId, uuid: pageUuid, position } = snapshot;
    cursorCache.set(cursorKey(vaultId, pageUuid), position);
    try {
      await invoke('cursor_position_save', { vaultId, uuid: pageUuid, ...position });
    } catch (e) {
      console.error('Failed to save the cursor position:', e);
    }
  }

  function saveCursorPosition() {
    return persistCursor(captureCursor());
  }

  async function loadCursor(vaultId: string, pageUuid: string): Promise<SavedCursorPosition | null> {
    const cached = cursorCache.get(cursorKey(vaultId, pageUuid));
    if (cached) return cached;
    try {
      return await invoke<SavedCursorPosition | null>('cursor_position_load', { vaultId, uuid: pageUuid });
    } catch (e) {
      console.error('Failed to load the cursor position:', e);
      return null;
    }
  }

  async function restoreCursor(pageUuid: string, token: number) {
    const vaultId = loadedVaultId;
    const saved = vaultId ? await loadCursor(vaultId, pageUuid) : null;

    await tick();
    // Carta sizes the textarea to its content in an animation frame after the
    // value lands; wait two frames so the scroll height is final.
    for (let i = 0; i < 2; i += 1) await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    if (token !== restoreToken) return;
    const textarea = carta.input?.textarea;
    if (!textarea) return;

    const target = resolveCursorPosition(textarea.value, saved);
    focusTextareaAtOffset(textarea, target.offset);
    const scroller = editorScroller();
    if (scroller) {
      scroller.scrollTop = target.scrollTop === 'end' ? scroller.scrollHeight : target.scrollTop;
      // The caret was placed without scrolling; the stored offset may no longer
      // show its line (window or font size changed, lines added above it).
      const input = carta.input;
      if (input) {
        const caretTop = caretTopInText(input);
        scroller.scrollTop = scrollTopShowingCaret(scroller.scrollTop, scroller.clientHeight, caretTop, input.getRowHeight());
      }
    }
    // Loading the page is not an edit: drop a save it armed.
    if (cursorSaveTimer) clearTimeout(cursorSaveTimer);
    cursorSaveTimer = null;
    restoredUuid = pageUuid;
  }

  // Entering write mode, or opening another page while in it, restores the caret.
  $effect(() => {
    const tab = app.editorTab;
    const pageUuid = loadedUuid;
    restoredUuid = null;
    const token = ++restoreToken;
    if (tab !== 'write' || !pageUuid) return;
    untrack(() => void restoreCursor(pageUuid, token));
  });

  // Also save 1 s after the last edit, so a crash loses little. Content set by
  // a page load lands before the restore and arms nothing, so pages that were
  // only viewed get no entry.
  $effect(() => {
    void content;
    untrack(() => {
      if (app.editorTab !== 'write' || !loadedUuid || restoredUuid !== loadedUuid) return;
      if (cursorSaveTimer) clearTimeout(cursorSaveTimer);
      cursorSaveTimer = setTimeout(() => void saveCursorPosition(), 1000);
    });
  });

  // ── Outline (spec `desktop-outline-panel`) ─────────────────────────────────

  // The Outline reads the live text, so it follows typing without a save.
  $effect(() => {
    app.pageText = content;
    app.pageTextUuid = loadedUuid;
  });
  $effect(() => () => {
    app.pageText = '';
    app.pageTextUuid = null;
    app.outlineCurrent = -1;
  });

  /** The rendered headings of the preview, in document order. */
  function previewHeadings(): HTMLElement[] {
    const surface = editorContainer?.querySelector('.carta-renderer');
    return surface ? Array.from(surface.querySelectorAll<HTMLElement>('h1, h2, h3, h4, h5, h6')) : [];
  }

  /** Level and text of rendered headings, for pairing them with the parsed ones. */
  function renderedInfo(els: HTMLElement[]): RenderedHeading[] {
    return els.map((el) => ({ level: Number(el.tagName.slice(1)), text: el.textContent ?? '' }));
  }

  /** The nearest ancestor of `el` that scrolls, or null. */
  function scrollParent(el: HTMLElement): HTMLElement | null {
    for (let node = el.parentElement; node; node = node.parentElement) {
      const overflowY = getComputedStyle(node).overflowY;
      if ((overflowY === 'auto' || overflowY === 'scroll') && node.scrollHeight > node.clientHeight) return node;
    }
    return null;
  }

  const jumpHold = createJumpHold();
  // Where the jump left its scroller once the scroll settled. A later scroll
  // away from there that no wheel, key or pointer event announced (a script,
  // an anchor, an assistive tool) is the user's too and ends the hold.
  let jumpSettled: { scroller: Element; top: number } | null = null;

  function settleJump(find: () => Element | null) {
    jumpSettled = null;
    requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        const scroller = find();
        if (scroller) jumpSettled = { scroller, top: scroller.scrollTop };
      }),
    );
  }

  /** Scroll so heading `index` is at the top; in edit mode the cursor moves there too. */
  function jumpToHeading(index: number) {
    const headings = headingsFor(content);
    const heading = headings[index];
    if (!heading) return;
    // The jump wins over any measurement still pending: it would be taken
    // before the scroll settled, or on a page too short to scroll at all.
    if (outlineTimer) clearTimeout(outlineTimer);
    outlineTimer = null;
    jumpHold.hold(index);
    app.outlineCurrent = index;
    if (app.editorTab === 'preview') {
      const rendered = previewHeadings();
      // Equal counts pair by position; a mismatch (raw HTML headings) pairs by level and text.
      const pair = pairRenderedHeadings(headings, renderedInfo(rendered)).find((p) => p.heading === index);
      if (pair) {
        const target = rendered[pair.rendered];
        target.scrollIntoView({ block: 'start' });
        settleJump(() => scrollParent(target));
      }
      return;
    }
    const input = carta.input;
    const textarea = input?.textarea;
    if (!input || !textarea) return;
    focusTextareaAtOffset(textarea, heading.offset);
    const scroller = editorScroller();
    if (scroller) scroller.scrollTop = textTopAtOffset(input, heading.offset);
    settleJump(() => scroller ?? null);
  }

  /** The heading whose section holds the top of the visible editor area. */
  function topHeadingIndex(): number {
    const headings = headingsFor(content);
    if (headings.length === 0) return -1;
    if (app.editorTab === 'preview') {
      const rendered = previewHeadings();
      const pairs = pairRenderedHeadings(headings, renderedInfo(rendered));
      if (pairs.length === 0) return -1;
      const scroller = scrollParent(rendered[pairs[0].rendered]);
      const edge = scroller ? scroller.getBoundingClientRect().top : 0;
      const found = currentHeadingIndex(
        pairs.length,
        (i) => rendered[pairs[i].rendered].getBoundingClientRect().top - edge,
        8,
      );
      return found < 0 ? -1 : pairs[found].heading;
    }
    const input = carta.input;
    const scroller = editorScroller();
    if (!input || !scroller) return -1;
    return currentHeadingIndex(headings.length, (i) => textTopAtOffset(input, headings[i].offset), scroller.scrollTop + 4);
  }

  // Measuring a long page is not free: while the Outline is selected, one
  // update runs shortly after the last scroll or edit.
  const OUTLINE_CURRENT_DELAY_MS = 120;
  let outlineTimer: ReturnType<typeof setTimeout> | null = null;

  function scheduleOutlineCurrent() {
    if (app.rightPanelTab !== 'outline' || outlineTimer) return;
    outlineTimer = setTimeout(() => {
      outlineTimer = null;
      app.outlineCurrent = app.rightPanelTab === 'outline' ? jumpHold.resolve(topHeadingIndex()) : -1;
    }, OUTLINE_CURRENT_DELAY_MS);
  }

  $effect(() => {
    const container = editorContainer;
    if (!container) return;
    // `scroll` does not bubble; capture sees the editor's and the preview's own scrollers.
    const onScroll = (e: Event) => {
      const settled = jumpSettled;
      if (settled && e.target === settled.scroller && Math.abs(settled.scroller.scrollTop - settled.top) > 2) {
        jumpHold.release();
        jumpSettled = null;
      }
      scheduleOutlineCurrent();
    };
    container.addEventListener('scroll', onScroll, true);
    // The user taking over (wheel, touch, scrollbar, keys) ends a jump's hold.
    const onUser = () => {
      jumpHold.release();
      scheduleOutlineCurrent();
    };
    const userEvents = ['wheel', 'touchmove', 'pointerdown', 'keydown'] as const;
    for (const type of userEvents) container.addEventListener(type, onUser, { capture: true, passive: true });
    const removeBridge = setOutlineBridge({ jumpTo: jumpToHeading });
    return () => {
      container.removeEventListener('scroll', onScroll, true);
      for (const type of userEvents) container.removeEventListener(type, onUser, true);
      removeBridge();
      if (outlineTimer) clearTimeout(outlineTimer);
      outlineTimer = null;
    };
  });

  // The tab opened, the text changed or the mode switched: mark again.
  $effect(() => {
    app.rightPanelTab;
    app.editorTab;
    void content;
    void loadedUuid;
    untrack(() => {
      jumpHold.release();
      scheduleOutlineCurrent();
    });
  });

  onMount(() => {
    // Leaving the page without a page change (vault switch, quit): write the
    // edits still waiting for their save, and where editing ended.
    const leave = async () => {
      await Promise.all([autosave.flush(), saveCursorPosition()]);
    };
    const stopLeaveHook = onEditorLeave(leave);
    // Closing the window by the window manager (the app's own quit runs the
    // leave hook). Bounded, so a stuck save never keeps the window open.
    let stopCloseListener: (() => void) | null = null;
    let disposed = false;
    void getCurrentWindow()
      .onCloseRequested(async () => {
        await Promise.race([leave(), new Promise((resolve) => setTimeout(resolve, CLOSE_SAVE_TIMEOUT_MS))]);
      })
      .then((stop) => {
        if (disposed) stop();
        else stopCloseListener = stop;
      })
      .catch((e) => console.error('Failed to watch window close:', e));
    return () => {
      disposed = true;
      stopLeaveHook();
      stopCloseListener?.();
      if (cursorSaveTimer) clearTimeout(cursorSaveTimer);
    };
  });

  // Handle 'i' and 'Esc' keys for mode switching
  $effect(() => {
    function handleKeydown(e: KeyboardEvent) {
      // Ignore if modal is open
      if (app.showSearch || app.showExitPrompt || app.showSparqlConsole) return;

      const modI = isModKey(e, 'i');

      if (isInputFocused()) {
        // If in editor, only handle Escape to switch to preview
        if (e.key === 'Escape' && app.editorTab === 'write') {
          // An open link picker takes Escape for itself.
          if (carta.input?.textarea?.dataset.linkPicker === 'open') return;
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
    // Capture before the switch: the write view is rebuilt on the tab change.
    if (app.editorTab === 'write') void saveCursorPosition();
    app.editorTab = app.editorTab === 'write' ? 'preview' : 'write';
  }

  // `Ctrl+E` and the toolbar toggle switch mode through the store; route them
  // here so the caret is stored before the write view goes away.
  $effect(() => onEditorModeToggle(toggleMode));

  async function loadCurrentPageFromDisk() {
    const vault = activeVault();
    if (!vault || !app.currentUuid) return;

    const nextContent = await invoke<string>('page_read', { uuid: app.currentUuid });
    logPageContentDiff(`[page-change-diff] ${app.currentUuid}`, content, nextContent);
    app.currentContent = nextContent;
    baselineContent = nextContent;
    content = nextContent;
    showConflictPrompt = false;
    previewDiffFile = null;
    lastConflictKey = '';
  }

  async function previewCurrentPageDiff() {
    const vault = activeVault();
    if (!vault || !app.currentUuid) return;

    previewDiffFile = await loadPageDiffFile(
      () => invoke<string>('page_read', { uuid: app.currentUuid! }),
      content,
      `${app.currentUuid}/page.md`,
    );
  }

  function suppressNextOwnPageChange() {
    const vault = activeVault();
    if (!vault) return;
    suppressedConflictKey = pageChangeKey(vault.id, `${vault.path}/${uuid}/page.md`);
  }

  async function overwriteCurrentPage() {
    suppressNextOwnPageChange();
    showConflictPrompt = false;
    previewDiffFile = null;
    lastConflictKey = '';
    await savePage(uuid, content);
  }
</script>

<div
  class="page-editor"
  class:write-mode={app.editorTab === 'write'}
  data-caret-restored={restoredUuid ?? ''}
  bind:this={editorContainer}
>

  {#key editorKey}
    {#if app.editorTab === 'write' && plainInput}
      <PlainMarkdownInput
        {carta}
        bind:value={content}
        theme={mode.current === 'dark' ? 'dark' : 'light'}
      />
    {:else}
      <MarkdownEditor
        {carta}
        bind:value={content}
        selectedTab={app.editorTab}
        mode="tabs"
        theme={mode.current === 'dark' ? 'dark' : 'light'}
        highlightDelay={OVERLAY_HIGHLIGHT_DELAY_MS}
      />
    {/if}
  {/key}

  {#if dropIndicator}
    <div class="drop-target" aria-hidden="true"></div>
    {#if dropIndicator.height > 0}
      <div
        class="drop-caret"
        aria-hidden="true"
        style:left="{dropIndicator.left}px"
        style:top="{dropIndicator.top}px"
        style:height="{dropIndicator.height}px"
      ></div>
    {/if}
  {/if}

  {#if showConflictPrompt}
    <ExternalChangeDialog
      onLoad={loadCurrentPageFromDisk}
      onOverwrite={overwriteCurrentPage}
      onPreview={previewCurrentPageDiff}
      diffFile={previewDiffFile}
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

  /* No bar of the editor's own above the page (spec `desktop-window-toolbar`,
     "No stacked bars"): in preview Carta's toolbar would be an empty strip, so
     it is hidden. In write mode it holds only the formatting buttons (Bold,
     Italic, ...), which stay; see design.md D2. */
  .page-editor:not(.write-mode) :global(.carta-toolbar) {
    display: none !important;
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



  /* Drop feedback: the editor outlined, a caret where the reference goes. */
  .page-editor > .drop-target {
    position: absolute;
    inset: 0;
    border: 2px dashed var(--primary);
    border-radius: var(--radius);
    background: color-mix(in oklab, var(--primary) 6%, transparent);
    pointer-events: none;
    z-index: 50;
  }

  .page-editor > .drop-caret {
    position: absolute;
    width: 2px;
    background: var(--primary);
    pointer-events: none;
    z-index: 51;
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
    /* The textarea re-measures its height on every keystroke, which made the
       scrollbar flicker while typing. Scrolling (wheel, keys, touch) still
       works; only the bar is hidden. */
    scrollbar-width: none;
  }

  .page-editor :global(.carta-input::-webkit-scrollbar) {
    display: none;
  }

  .page-editor :global(.carta-renderer) {
    height: 100%;
    overflow-y: auto;
    background: transparent;
  }

  .page-editor :global(.carta-renderer ul.contains-task-list),
  .page-editor :global(.carta-renderer ul.task-list-item) {
    list-style: none;
    padding-left: 1.4em;
  }

  .page-editor :global(.carta-renderer input.task-list-checkbox) {
    width: 1.1em;
    height: 1.1em;
    margin: 0 0.5em 0 0;
    vertical-align: text-bottom;
    flex: 0 0 auto;
    transform: translateY(0.02em);
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

  /* Mentions are typed text, not links: a tint and a dotted underline, no link colour. */
  :global(.carta-renderer span.mention) {
    color: var(--fg);
    background: color-mix(in oklab, var(--primary) 12%, transparent);
    border-bottom: 1px dotted var(--primary);
    border-radius: calc(var(--radius) / 2);
    padding: 0 0.15em;
    cursor: help;
  }

  :global(.carta-renderer span.mention:hover) {
    background: color-mix(in oklab, var(--primary) 22%, transparent);
  }

  /* Unknown prefix: same red as a broken wikilink. */
  :global(.carta-renderer span.mention.unresolved) {
    background: color-mix(in oklab, #dc2626 10%, transparent);
    border-bottom: none;
    text-decoration: underline wavy #dc2626;
    text-decoration-thickness: 1px;
    text-underline-offset: 0.2em;
  }
</style>
