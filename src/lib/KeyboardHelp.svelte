<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import { app, closeAllDialogs } from './stores.svelte';
  import { isMac } from './hotkeys';

  let { onClose }: { onClose: () => void } = $props();

  const mod = isMac ? '⌘' : 'Ctrl';

  const shortcuts = [
    { keys: [mod, 'P'], desc: 'Open page search' },
    { keys: [mod, 'N'], desc: 'Create new page' },
    { keys: [mod, 'T'], desc: 'Toggle light/dark theme' },
    { keys: [mod, 'O'], desc: 'Add new vault' },
    { keys: [mod, 'Q'], desc: 'Quit application' },
    { keys: ['F1'], desc: 'Show this help' },
    { keys: ['Esc'], desc: 'Close active dialog or modal' },
    { keys: ['r'], desc: 'Rename current page (Preview mode)' },
    { keys: ['Del'], desc: 'Delete current page (Preview mode)' },
    { keys: ['↓'], desc: 'Next page in sidebar' },
    { keys: ['j'], desc: 'Next page (Vim-style)' },
    { keys: ['↑'], desc: 'Previous page in sidebar' },
    { keys: ['k'], desc: 'Previous page (Vim-style)' },
    { keys: ['Tab'], desc: 'Cycle pages in sidebar' },
  ];
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div 
  class="modal-backdrop" 
  onmousedown={(e) => e.target === e.currentTarget && onClose()}
  transition:fade={{ duration: 150 }}
>
  <div 
    class="modal-content" 
    transition:fly={{ y: 20, duration: 200 }}
  >
    <header class="modal-header">
      <h2>Keyboard Shortcuts</h2>
      <button class="close-btn" onclick={onClose} title="Close (Esc)">✕</button>
    </header>

    <div class="shortcuts-grid">
      {#each shortcuts as { keys, desc }}
        <div class="shortcut-row">
          <div class="keys">
            {#each keys as key, i}
              <kbd>{key}</kbd>
              {#if i < keys.length - 1}<span class="plus">+</span>{/if}
            {/each}
          </div>
          <div class="desc">{desc}</div>
        </div>
      {/each}
    </div>

    <footer class="modal-footer">
      <p>Press <kbd>Esc</kbd> to close</p>
    </footer>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(2px);
  }

  .modal-content {
    background: var(--bg);
    width: 100%;
    max-width: 450px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    box-shadow: 0 20px 25px -5px rgb(0 0 0 / 0.1), 0 8px 10px -6px rgb(0 0 0 / 0.1);
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid var(--border);
  }

  .modal-header h2 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
  }

  .close-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--muted-foreground);
    padding: 0.25rem;
    border-radius: var(--radius);
  }

  .close-btn:hover {
    background: var(--accent-hover);
    color: var(--fg);
  }

  .shortcuts-grid {
    padding: 1rem 0;
    max-height: 60vh;
    overflow-y: auto;
  }

  .shortcut-row {
    display: flex;
    align-items: center;
    padding: 0.6rem 1.25rem;
    gap: 1.5rem;
  }

  .shortcut-row:hover {
    background: var(--accent-hover);
  }

  .keys {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    min-width: 100px;
    justify-content: flex-end;
  }

  .plus {
    font-size: 0.75rem;
    color: var(--muted-foreground);
  }

  kbd {
    display: inline-block;
    padding: 0.2rem 0.4rem;
    font-size: 0.75rem;
    font-family: inherit;
    line-height: 1;
    color: var(--fg);
    background: var(--accent);
    border: 1px solid var(--border);
    border-radius: 4px;
    box-shadow: 0 1px 0 var(--border);
    min-width: 1.5rem;
    text-align: center;
  }

  .desc {
    font-size: 0.9rem;
    color: var(--fg);
  }

  .modal-footer {
    padding: 0.75rem 1.25rem;
    background: var(--accent);
    border-top: 1px solid var(--border);
    text-align: center;
  }

  .modal-footer p {
    margin: 0;
    font-size: 0.8rem;
    color: var(--muted-foreground);
  }
</style>
