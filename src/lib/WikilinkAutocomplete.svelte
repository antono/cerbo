<script lang="ts">
  import type { Carta } from 'carta-md';

  /**
   * WikilinkAutocomplete — injected into Carta's input area.
   * Watches the textarea for `[[` patterns and shows a filtered dropdown.
   *
   * Props injected by Carta: carta
   * Props from plugin options: getPages
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
  let triggerStart = $state(-1); // caret position where [[ was typed

  // Dropdown anchor (pixel offset inside editor container)
  let anchorX = $state(0);
  let anchorY = $state(0);

  // ── Derived page suggestions ─────────────────────────────────────────────────

  const suggestions = $derived(() => {
    const pages = getPages();
    if (!query) return pages.slice(0, 10);
    const q = query.toLowerCase();
    return pages.filter((s) => s.toLowerCase().includes(q)).slice(0, 10);
  });

  // ── Helpers ──────────────────────────────────────────────────────────────────

  /** Convert a slug to a display title (slug → Title Case). */
  function slugToTitle(slug: string): string {
    return slug
      .split('-')
      .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
      .join(' ');
  }

  /** Insert the selected page title and close the dropdown. */
  function insertSuggestion(slug: string) {
    const input = carta.input;
    if (!input) return;
    const textarea = input.textarea;
    const pos = textarea.selectionStart;
    const title = slugToTitle(slug);
    const before = textarea.value.slice(0, triggerStart);
    const after = textarea.value.slice(pos);
    const insertion = `[[${title}]]`;
    const newValue = before + insertion + after;
    const newCaret = triggerStart + insertion.length;

    // Mutate via InputEnhancer to preserve undo history
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

  // ── Keyboard handler ─────────────────────────────────────────────────────────

  function handleKeyDown(ev: KeyboardEvent) {
    if (!open) return;
    const list = suggestions();
    if (ev.key === 'ArrowDown') {
      ev.preventDefault();
      selected = (selected + 1) % list.length;
    } else if (ev.key === 'ArrowUp') {
      ev.preventDefault();
      selected = (selected - 1 + list.length) % list.length;
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

  // ── Textarea input watcher ────────────────────────────────────────────────────

  function handleInput() {
    const input = carta.input;
    if (!input) return;
    const textarea = input.textarea;
    const pos = textarea.selectionStart;
    const text = textarea.value.slice(0, pos);

    // Find the last [[ before cursor that hasn't been closed yet
    const match = /\[\[([^\][\n]*)$/.exec(text);
    if (match) {
      const start = match.index;
      if (!open) {
        // Compute anchor position from caret
        const rect = textarea.getBoundingClientRect();
        const containerRect = textarea.parentElement?.getBoundingClientRect() ?? rect;
        // Approximate caret pixel position
        anchorX = 0;
        anchorY = 0; // Will be positioned via CSS near caret
        triggerStart = start;
        open = true;
        selected = 0;
      }
      query = match[1];
    } else {
      if (open) close();
    }
  }

  // ── Mount / unmount ──────────────────────────────────────────────────────────

  $effect(() => {
    const input = carta.input;
    if (!input) return;
    const textarea = input.textarea;
    textarea.addEventListener('input', handleInput);
    textarea.addEventListener('keydown', handleKeyDown, true); // capture phase
    textarea.addEventListener('blur', () => { if (open) close(); });

    return () => {
      textarea.removeEventListener('input', handleInput);
      textarea.removeEventListener('keydown', handleKeyDown, true);
    };
  });
</script>

{#if open}
  {@const list = suggestions()}
  {#if list.length > 0}
    <!-- Positioned at the bottom-left of the textarea container -->
    <div
      class="wikilink-autocomplete"
      role="listbox"
      aria-label="Page suggestions"
    >
      {#each list as slug, i}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          class="wikilink-autocomplete-item"
          class:selected={i === selected}
          role="option"
          aria-selected={i === selected}
          onmousedown={(e) => { e.preventDefault(); insertSuggestion(slug); }}
        >
          {slugToTitle(slug)}
        </div>
      {/each}
    </div>
  {/if}
{/if}

<style>
  .wikilink-autocomplete {
    position: absolute;
    bottom: 0;
    left: 0;
    z-index: 100;
    background: var(--carta-autocomplete-bg, hsl(var(--background)));
    border: 1px solid var(--carta-autocomplete-border, hsl(var(--border)));
    border-radius: 0.375rem;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    min-width: 200px;
    max-height: 240px;
    overflow-y: auto;
    font-size: 0.875rem;
  }

  .wikilink-autocomplete-item {
    padding: 0.375rem 0.75rem;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--carta-autocomplete-text, hsl(var(--foreground)));
  }

  .wikilink-autocomplete-item:hover,
  .wikilink-autocomplete-item.selected {
    background: var(--carta-autocomplete-hover, hsl(var(--accent)));
    color: var(--carta-autocomplete-hover-text, hsl(var(--accent-foreground)));
  }
</style>
