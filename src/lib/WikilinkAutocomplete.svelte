<script lang="ts">
  import type { Carta } from 'carta-md';
  import type { VaultObject } from './stores.svelte';

  /**
   * WikilinkAutocomplete — injected into Carta's input area.
   * Watches the textarea for `[[` patterns and shows a filtered dropdown.
   * Inserts cerbo:// UUID-based links instead of wikilinks.
   */

  let {
    carta,
    getVaultObjects,
  }: {
    carta: Carta;
    getVaultObjects: () => VaultObject[];
  } = $props();

  // ── State ────────────────────────────────────────────────────────────────────

  let open = $state(false);
  let query = $state('');
  let selected = $state(0);
  let triggerStart = $state(-1);
  let triggerType = $state<'wikilink' | 'cerbo-link'>('wikilink');

  // Dropdown position
  let anchorX = $state(0);
  let anchorY = $state(0);

  // ── Derived ──────────────────────────────────────────────────────────────────

  const suggestions = $derived(() => {
    const objects = getVaultObjects();
    if (!query) {
      return objects.slice().sort((a, b) => a.title.localeCompare(b.title)).slice(0, 10);
    }
    const q = query.toLowerCase();
    return objects
      .filter((o) => o.title.toLowerCase().includes(q))
      .sort((a, b) => {
        const aTitle = a.title.toLowerCase();
        const bTitle = b.title.toLowerCase();
        const aStarts = aTitle.startsWith(q);
        const bStarts = bTitle.startsWith(q);
        if (aStarts !== bStarts) return bStarts ? 1 : -1;
        return aTitle.localeCompare(bTitle);
      })
      .slice(0, 10);
  });

  // ── Caret Position Helper ───────────────────────────────────────────────────

  /**
   * A simplified version of getting caret coordinates in a textarea.
   * It creates a "mirror" element to calculate the pixel offset.
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
    div.textContent = element.value.substring(0, position);

    // The span represents the caret
    const span = document.createElement('span');
    span.textContent = element.value.substring(position) || '.';
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

  // ── Helpers ──────────────────────────────────────────────────────────────────

  function insertSuggestion(obj: VaultObject) {
    const input = carta.input;
    if (!input) return;
    const textarea = input.textarea;
    const pos = textarea.selectionStart;

    let insertion = '';
    if (triggerType === 'wikilink') {
      insertion = `[${obj.title}](cerbo://objects/${obj.uuid})`;
    } else if (triggerType === 'cerbo-link') {
      insertion = `cerbo://objects/${obj.uuid})`;
    }

    textarea.focus();
    textarea.setSelectionRange(triggerStart, pos);
    document.execCommand('insertText', false, insertion);

    close();
  }

  function close() {
    open = false;
    query = '';
    selected = 0;
    triggerStart = -1;
    triggerType = 'wikilink';
  }

  // ── Handlers ─────────────────────────────────────────────────────────────────

  function handleKeyDown(ev: KeyboardEvent) {
    if (!open) return;
    const list = suggestions();
    if (ev.key === 'ArrowDown') {
      ev.preventDefault();
      selected = (selected + 1) % (list.length || 1);
    } else if (ev.key === 'ArrowUp') {
      ev.preventDefault();
      selected = (selected - 1 + (list.length || 1)) % (list.length || 1);
    } else if (ev.key === 'Enter' || ev.key === 'Tab') {
      if (list.length > 0) {
        ev.preventDefault();
        insertSuggestion(list[selected]);
      }
    } else if (ev.key === 'Escape') {
      ev.stopPropagation();
      close();
    }
  }

  function handleInput() {
    const input = carta.input;
    if (!input) return;
    const textarea = input.textarea;
    const pos = textarea.selectionStart;
    const text = textarea.value.slice(0, pos);

    // Try wikilink pattern: [[...
    const wikiMatch = /\[\[([^\][\n]*)$/.exec(text);
    if (wikiMatch) {
      triggerType = 'wikilink';
      const start = wikiMatch.index;
      triggerStart = start;

      const coords = getCaretCoordinates(textarea, pos);
      anchorX = coords.x;
      anchorY = coords.y;

      open = true;
      selected = 0;
      query = wikiMatch[1];
      return;
    }

    // Try cerbo link pattern: [text](cerbo://...
    const cerboMatch = /\[([^\]]*)\]\(cerbo:\/\/([^)]*)\)$/.exec(text);
    if (cerboMatch) {
      triggerType = 'cerbo-link';
      const start = text.lastIndexOf('[');
      triggerStart = start + cerboMatch[1].length + 3; // Position after ](cerbo://

      const coords = getCaretCoordinates(textarea, pos);
      anchorX = coords.x;
      anchorY = coords.y;

      open = true;
      selected = 0;
      query = cerboMatch[2]; // Query after cerbo://
      return;
    }

    if (open) close();
  }

  $effect(() => {
    const input = carta.input;
    if (!input) return;
    const textarea = input.textarea;
    textarea.addEventListener('input', handleInput);
    textarea.addEventListener('keydown', handleKeyDown, true);
    textarea.addEventListener('blur', () => { 
      // Small timeout to allow click on suggestions
      setTimeout(() => { if (open) close(); }, 150); 
    });

    return () => {
      textarea.removeEventListener('input', handleInput);
      textarea.removeEventListener('keydown', handleKeyDown, true);
    };
  });
</script>

{#if open}
  {@const list = suggestions()}
  {#if list.length > 0}
    <div
      class="wikilink-autocomplete"
      role="listbox"
      aria-label="Page suggestions"
      style:left="{anchorX}px"
      style:top="{anchorY}px"
    >
      {#each list as obj, i}
        <button
          class="wikilink-autocomplete-item"
          class:selected={i === selected}
          role="option"
          aria-selected={i === selected}
          onmousedown={(e) => { e.preventDefault(); insertSuggestion(obj); }}
          title={obj.uuid}
        >
          <span class="object-type">[{obj.object_type}]</span>
          <span class="object-title">{obj.title}</span>
        </button>
      {/each}
    </div>
  {/if}
{/if}

<style>
  .wikilink-autocomplete {
    position: absolute;
    z-index: 1000;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
    min-width: 200px;
    max-height: 240px;
    overflow-y: auto;
    font-size: 0.875rem;
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
</style>
