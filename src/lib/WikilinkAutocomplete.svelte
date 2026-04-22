<script lang="ts">
  import type { Carta } from 'carta-md';

  /**
   * WikilinkAutocomplete — injected into Carta's input area.
   * Watches the textarea for `[[` patterns and shows a filtered dropdown.
   */

  let {
    carta,
    getPages,
  }: {
    carta: Carta;
    getPages: () => string[];
  } = $props();

  // ── State ────────────────────────────────────────────────────────────────────

  let open = $state(false);
  let query = $state('');
  let selected = $state(0);
  let triggerStart = $state(-1);

  // Dropdown position
  let anchorX = $state(0);
  let anchorY = $state(0);

  // ── Derived ──────────────────────────────────────────────────────────────────

  const suggestions = $derived(() => {
    const pages = getPages();
    if (!query) return pages.slice(0, 10);
    const q = query.toLowerCase();
    return pages.filter((s) => s.toLowerCase().includes(q)).slice(0, 10);
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

  function slugToTitle(slug: string): string {
    return slug
      .split('-')
      .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
      .join(' ');
  }

  function insertSuggestion(slug: string) {
    const input = carta.input;
    if (!input) return;
    const textarea = input.textarea;
    const pos = textarea.selectionStart;
    const title = slugToTitle(slug);
    const insertion = `[[${title}]]`;

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

    const match = /\[\[([^\][\n]*)$/.exec(text);
    if (match) {
      const start = match.index;
      triggerStart = start;
      
      const coords = getCaretCoordinates(textarea, pos);
      anchorX = coords.x;
      anchorY = coords.y;
      
      open = true;
      selected = 0;
      query = match[1];
    } else {
      if (open) close();
    }
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
      {#each list as slug, i}
        <button
          class="wikilink-autocomplete-item"
          class:selected={i === selected}
          role="option"
          aria-selected={i === selected}
          onmousedown={(e) => { e.preventDefault(); insertSuggestion(slug); }}
        >
          {slugToTitle(slug)}
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
    display: block;
    width: 100%;
    text-align: left;
    padding: 0.375rem 0.75rem;
    border: none;
    background: none;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--fg);
  }

  .wikilink-autocomplete-item:hover,
  .wikilink-autocomplete-item.selected {
    background: var(--accent);
    color: var(--fg);
  }
</style>
