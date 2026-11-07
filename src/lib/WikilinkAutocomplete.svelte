<script lang="ts">
  import type { Carta } from 'carta-md';
  import type { VaultObject } from './stores.svelte';
  import {
    buildLinkIndex,
    clampSelection,
    createFenceMemo,
    detectTrigger,
    insertionFor,
    moveSelection,
    searchLinkIndex,
    type LinkCandidate,
    type LinkIndex,
    type LinkTriggerKind,
  } from './link-picker';

  /**
   * WikilinkAutocomplete — the link picker injected into Carta's input area
   * (design Decision 2). Opens on `[`, `[[` and `](cerbo://`, filters vault
   * objects by the text typed after the trigger and inserts cerbo:// UUID
   * links. Focus stays in the textarea: the search field only mirrors the
   * query, so typing, Backspace and IME work as usual and dismissing leaves
   * the typed text alone.
   */

  let {
    carta,
    getVaultObjects,
    getPagePaths = () => [],
  }: {
    carta: Carta;
    getVaultObjects: () => VaultObject[];
    /** Pages with their virtual path; the object list does not carry it. */
    getPagePaths?: () => { uuid: string; virtualPath: string | null }[];
  } = $props();

  // ── State ────────────────────────────────────────────────────────────────────

  let open = $state(false);
  let query = $state('');
  let selected = $state(0);
  let triggerStart = $state(-1);
  let triggerKind = $state<LinkTriggerKind>('link');
  /** Start of the trigger closed with Escape; it stays closed while typing on. */
  let dismissedStart = -1;
  /** Set while our own insertion fires `input`, so it does not reopen the picker. */
  let inserting = false;
  /** Fenced-code answer for the caret's line; dropped whenever focus leaves. */
  const fenceMemo = createFenceMemo();

  // Dropdown position
  let anchorX = $state(0);
  let anchorY = $state(0);
  let positionFrame = 0;

  let listEl = $state<HTMLDivElement | null>(null);

  // ── Derived ──────────────────────────────────────────────────────────────────

  // ── Search index ─────────────────────────────────────────────────────────────

  // Built eagerly in idle time after the object or page list changes, so no
  // keystroke pays for it (~75 ms at 13 700 objects). Opening the picker before
  // the idle build ran builds it at once (`ensureIndex`).
  let index = $state.raw<LinkIndex>(buildLinkIndex([]));
  let pendingBuild: (() => LinkIndex) | null = null;

  const idle: (cb: () => void) => number =
    typeof requestIdleCallback === 'function'
      ? (cb) => requestIdleCallback(cb, { timeout: 1000 })
      : (cb) => window.setTimeout(cb, 50);
  const cancelIdle: (handle: number) => void =
    typeof cancelIdleCallback === 'function' ? cancelIdleCallback : window.clearTimeout;

  function ensureIndex() {
    if (!pendingBuild) return;
    const build = pendingBuild;
    pendingBuild = null;
    index = build();
  }

  $effect(() => {
    const objects = getVaultObjects();
    const paths = new Map(getPagePaths().map((p) => [p.uuid, p.virtualPath]));
    pendingBuild = () => buildLinkIndex(objects, (uuid) => paths.get(uuid));
    const handle = idle(ensureIndex);
    return () => cancelIdle(handle);
  });

  const results = $derived<LinkCandidate[]>(open ? searchLinkIndex(index, query) : []);

  // A rebuilt index can shrink the list under the selection.
  $effect(() => {
    const clamped = clampSelection(selected, results.length);
    if (clamped !== selected) selected = clamped;
  });

  // ── Caret Position Helper ───────────────────────────────────────────────────

  /**
   * A simplified version of getting caret coordinates in a textarea.
   * It creates a "mirror" element to calculate the pixel offset. The mirror
   * holds the whole text before `position`, because wrapped lines above decide
   * its height; that costs one layout per opened trigger (see schedulePosition).
   */
  function getCaretCoordinates(element: HTMLTextAreaElement, position: number) {
    const div = document.createElement('div');
    const style = window.getComputedStyle(element);

    // Copy essential styles
    const properties = [
      'fontFamily', 'fontSize', 'fontWeight', 'fontStyle', 'fontVariant',
      'textTransform', 'wordSpacing', 'letterSpacing', 'lineHeight',
      'paddingLeft', 'paddingTop', 'paddingRight', 'paddingBottom',
      'marginLeft', 'marginTop', 'marginRight', 'marginBottom',
      'borderLeftWidth', 'borderTopWidth', 'borderRightWidth', 'borderBottomWidth',
      'width', 'height', 'boxSizing', 'overflowX', 'overflowY', 'wordWrap', 'whiteSpace'
    ];

    properties.forEach(prop => {
      // @ts-ignore
      div.style[prop] = style[prop];
    });

    div.style.position = 'absolute';
    div.style.visibility = 'hidden';
    div.style.whiteSpace = 'pre-wrap';
    div.style.wordWrap = 'break-word';

    // Content up to caret
    const value = element.value;
    div.textContent = value.substring(0, position);

    // The span represents the caret; only the rest of its line affects wrapping.
    const span = document.createElement('span');
    const lineEnd = value.indexOf('\n', position);
    span.textContent = value.substring(position, lineEnd === -1 ? value.length : lineEnd) || '.';
    div.appendChild(span);

    document.body.appendChild(div);
    const { offsetLeft: spanX, offsetTop: spanY } = span;
    const lineHeight = parseInt(style.lineHeight);

    document.body.removeChild(div);

    return {
      x: spanX - element.scrollLeft,
      y: spanY - element.scrollTop + (isNaN(lineHeight) ? 20 : lineHeight)
    };
  }

  /**
   * Place the dropdown under the trigger in the next frame. The mirror forces a
   * layout, so it runs once per opened trigger and never on the keystroke path.
   */
  function schedulePosition() {
    if (positionFrame) return;
    positionFrame = requestAnimationFrame(() => {
      positionFrame = 0;
      const textarea = carta.input?.textarea;
      if (!open || !textarea) return;
      const coords = getCaretCoordinates(textarea, triggerStart);
      anchorX = coords.x;
      anchorY = coords.y;
    });
  }

  // ── Helpers ──────────────────────────────────────────────────────────────────

  function insertSuggestion(obj: LinkCandidate) {
    const input = carta.input;
    if (!input) return;
    const textarea = input.textarea;
    const pos = textarea.selectionStart;
    const start = triggerStart;
    const insertion = insertionFor(triggerKind, obj);

    close();
    textarea.focus();
    textarea.setSelectionRange(start, pos);
    inserting = true;
    try {
      // Keeps native undo; the caret ends after the inserted `)`.
      document.execCommand('insertText', false, insertion);
    } finally {
      inserting = false;
    }
  }

  function close() {
    open = false;
    query = '';
    selected = 0;
    triggerStart = -1;
    triggerKind = 'link';
    if (positionFrame) {
      cancelAnimationFrame(positionFrame);
      positionFrame = 0;
    }
  }

  /** Re-read the trigger at the caret; opens the picker only when `mayOpen`. */
  function sync(mayOpen: boolean) {
    const textarea = carta.input?.textarea;
    if (!textarea) return;
    const pos = textarea.selectionStart;
    const trigger =
      textarea.selectionEnd === pos
        ? detectTrigger(textarea.value, pos, { fenceMemo })
        : null;

    if (!trigger) {
      dismissedStart = -1;
      if (open) close();
      return;
    }
    if (trigger.start === dismissedStart) return;
    dismissedStart = -1;

    const isNew = !open || trigger.start !== triggerStart || trigger.kind !== triggerKind;
    if (isNew && !(mayOpen || open)) return;
    if (isNew && open && !mayOpen) {
      // The caret moved into another trigger: close rather than jump.
      close();
      return;
    }
    triggerKind = trigger.kind;
    triggerStart = trigger.start;
    query = trigger.query;
    selected = 0;
    if (isNew) {
      ensureIndex();
      open = true;
      schedulePosition();
    }
  }

  // ── Handlers ─────────────────────────────────────────────────────────────────

  function handleKeyDown(ev: KeyboardEvent) {
    if (!open || ev.isComposing) return;
    const list = results;
    if (ev.key === 'ArrowDown' || ev.key === 'ArrowUp') {
      ev.preventDefault();
      ev.stopImmediatePropagation();
      selected = moveSelection(selected, ev.key === 'ArrowDown' ? 1 : -1, list.length);
    } else if (ev.key === 'Enter' || ev.key === 'Tab') {
      if (list.length > 0) {
        // Carta must not also add a newline or indent.
        ev.preventDefault();
        ev.stopImmediatePropagation();
        insertSuggestion(list[clampSelection(selected, list.length)]);
      }
    } else if (ev.key === 'Escape') {
      ev.preventDefault();
      ev.stopImmediatePropagation();
      dismissedStart = triggerStart;
      close();
    }
  }

  function handleInput() {
    if (!inserting) sync(true);
  }

  const CARET_KEYS = new Set(['ArrowLeft', 'ArrowRight', 'Home', 'End', 'PageUp', 'PageDown']);

  /** Caret moved without typing (arrows, click): follow or close, never open. */
  function handleCaretMove(ev: Event) {
    if (!open) return;
    if (ev instanceof KeyboardEvent && !CARET_KEYS.has(ev.key)) return;
    sync(false);
  }

  function handleBlur() {
    // The text may change while focus is away (page switch).
    fenceMemo.last = null;
    // Small timeout to allow click on suggestions
    setTimeout(() => { if (open) close(); }, 150);
  }

  $effect(() => {
    const input = carta.input;
    if (!input) return;
    const textarea = input.textarea;
    textarea.addEventListener('input', handleInput);
    textarea.addEventListener('keydown', handleKeyDown, true);
    textarea.addEventListener('keyup', handleCaretMove);
    textarea.addEventListener('mouseup', handleCaretMove);
    textarea.addEventListener('blur', handleBlur);

    return () => {
      textarea.removeEventListener('input', handleInput);
      textarea.removeEventListener('keydown', handleKeyDown, true);
      textarea.removeEventListener('keyup', handleCaretMove);
      textarea.removeEventListener('mouseup', handleCaretMove);
      textarea.removeEventListener('blur', handleBlur);
      if (positionFrame) cancelAnimationFrame(positionFrame);
    };
  });

  // Tells the editor's window-level Escape handler (switch to preview) that
  // Escape belongs to the picker.
  $effect(() => {
    const textarea = carta.input?.textarea;
    if (!textarea) return;
    if (open) textarea.dataset.linkPicker = 'open';
    else delete textarea.dataset.linkPicker;
  });

  $effect(() => {
    if (!open || !listEl) return;
    listEl.querySelector<HTMLElement>(`[data-index="${selected}"]`)?.scrollIntoView({ block: 'nearest' });
  });
</script>

{#if open}
  <!-- Mouse down must not take focus from the textarea. -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="wikilink-autocomplete"
    style:left="{anchorX}px"
    style:top="{anchorY}px"
    onmousedown={(e) => e.preventDefault()}
  >
    <input
      class="link-picker-query"
      type="text"
      readonly
      tabindex="-1"
      value={query}
      placeholder="Search objects"
      aria-label="Link search"
    />
    <div class="link-picker-list" role="listbox" aria-label="Page suggestions" bind:this={listEl}>
      {#each results as obj, i (obj.uuid)}
        <button
          class="wikilink-autocomplete-item"
          class:selected={i === selected}
          role="option"
          tabindex="-1"
          data-index={i}
          aria-selected={i === selected}
          onmousedown={(e) => { e.preventDefault(); insertSuggestion(obj); }}
          title={obj.virtualPath ? `${obj.virtualPath}/${obj.title}` : obj.uuid}
        >
          <span class="object-type">[{obj.objectType}]</span>
          <span class="object-title">{obj.title}</span>
          {#if obj.virtualPath}
            <span class="object-path">{obj.virtualPath}</span>
          {/if}
        </button>
      {:else}
        <div class="link-picker-empty">No objects found</div>
      {/each}
    </div>
  </div>
{/if}

<style>
  .wikilink-autocomplete {
    position: absolute;
    z-index: 1000;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
    min-width: 240px;
    max-width: 420px;
    font-size: 0.875rem;
  }

  .link-picker-query {
    margin: 0.375rem;
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
    color: var(--fg);
    font: inherit;
    pointer-events: none;
  }

  .link-picker-list {
    max-height: 240px;
    overflow-y: auto;
  }

  .link-picker-empty {
    padding: 0.375rem 0.75rem;
    opacity: 0.7;
  }

  .wikilink-autocomplete-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    text-align: left;
    padding: 0.375rem 0.75rem;
    border: none;
    background: none;
    cursor: pointer;
    color: var(--fg);
    min-width: 0;
  }

  .wikilink-autocomplete-item:hover,
  .wikilink-autocomplete-item.selected {
    background: var(--accent);
    color: var(--fg);
  }

  .object-type {
    flex-shrink: 0;
    font-size: 0.75rem;
    opacity: 0.7;
    font-weight: 500;
  }

  .object-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .object-path {
    margin-left: auto;
    flex-shrink: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.75rem;
    opacity: 0.6;
  }
</style>
