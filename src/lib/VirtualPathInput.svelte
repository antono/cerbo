<script lang="ts">
  /**
   * The folder path field shared by the New Page and Move dialogs (design D7):
   * one input, suggestions from the folders that exist, and the backend's
   * validation rules mirrored client-side.
   */
  import { app } from './stores.svelte';
  import { folderPaths } from './page-tree';
  import { virtualPathError } from './virtual-path';

  let {
    value = $bindable(''),
    suggestionsOpen = $bindable(false),
    inputEl = $bindable<HTMLInputElement | null>(null),
    disabled = false,
    id = 'virtual-path',
    placeholder = 'notes/rust',
  }: {
    value?: string;
    suggestionsOpen?: boolean;
    inputEl?: HTMLInputElement | null;
    disabled?: boolean;
    id?: string;
    placeholder?: string;
  } = $props();

  const existing = $derived(folderPaths(app.pages, app.layout));
  const typed = $derived(value.trim());
  const suggestions = $derived(
    typed
      ? existing.filter((p) => p !== typed && p.toLowerCase().startsWith(typed.toLowerCase())).slice(0, 8)
      : [],
  );
  const error = $derived(virtualPathError(typed));
  let highlighted = $state(0);

  $effect(() => {
    if (suggestions.length === 0) suggestionsOpen = false;
    if (highlighted >= suggestions.length) highlighted = 0;
  });

  function onInput() {
    suggestionsOpen = suggestions.length > 0;
    highlighted = 0;
  }

  function pick(path: string) {
    value = path;
    suggestionsOpen = false;
    inputEl?.focus();
  }

  function onKeydown(e: KeyboardEvent) {
    if (!suggestionsOpen || suggestions.length === 0) return;
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      highlighted = (highlighted + 1) % suggestions.length;
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      highlighted = (highlighted - 1 + suggestions.length) % suggestions.length;
    } else if (e.key === 'Tab' || e.key === 'Enter') {
      e.preventDefault();
      pick(suggestions[highlighted]);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      suggestionsOpen = false;
    }
  }
</script>

<div class="path-input-wrapper">
  <input
    {id}
    bind:this={inputEl}
    bind:value
    oninput={onInput}
    onkeydown={onKeydown}
    onblur={() => setTimeout(() => (suggestionsOpen = false), 120)}
    {placeholder}
    class="dialog-input"
    class:invalid={error !== null}
    spellcheck="false"
    autocomplete="off"
    aria-invalid={error !== null}
    aria-autocomplete="list"
    aria-expanded={suggestionsOpen}
    {disabled}
  />
  {#if suggestionsOpen && suggestions.length > 0}
    <div class="suggestions" role="listbox">
      {#each suggestions as suggestion, i (suggestion)}
        <button
          type="button"
          class="suggestion-item"
          class:highlighted={i === highlighted}
          role="option"
          aria-selected={i === highlighted}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => pick(suggestion)}
          {disabled}
        >
          {suggestion}
        </button>
      {/each}
    </div>
  {/if}
</div>
{#if error}
  <div class="error-msg">{error}</div>
{/if}

<style>
  .path-input-wrapper {
    position: relative;
  }
  .dialog-input {
    width: 100%;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0.75rem 1rem;
    font-size: 1rem;
    color: var(--fg);
    outline: none;
    transition: border-color 0.2s;
  }
  .dialog-input:focus {
    border-color: var(--primary);
  }
  .dialog-input.invalid {
    border-color: #ef4444;
  }
  .suggestions {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    margin-top: 0.25rem;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.15);
    z-index: 10;
    max-height: 12rem;
    overflow-y: auto;
  }
  .suggestion-item {
    display: block;
    width: 100%;
    text-align: left;
    padding: 0.5rem 0.75rem;
    border: none;
    background: transparent;
    color: var(--fg);
    font-size: 0.9rem;
    cursor: pointer;
  }
  .suggestion-item:hover,
  .suggestion-item.highlighted {
    background: var(--accent-hover);
  }
  .error-msg {
    color: #ef4444;
    font-size: 0.85rem;
    margin-top: 0.25rem;
  }
</style>
