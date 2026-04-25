<script lang="ts">
  import { mode, setMode } from 'mode-watcher';
  import { Sun, Moon } from 'lucide-svelte';
  import { app, saveUiSettings } from '$lib/stores.svelte';

  function toggleTheme() {
    const next = mode.current === 'light' ? 'dark' : 'light';
    app.theme = next;
    setMode(next);
    saveUiSettings();
  }
</script>

<button
  class="theme-toggle"
  onclick={toggleTheme}
  title="Toggle theme"
  aria-label="Toggle theme"
>
  {#if app.theme === 'dark'}
    <Moon size={18} />
  {:else}
    <Sun size={18} />
  {/if}
</button>

<style>
  .theme-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    background: none;
    border: none;
    cursor: pointer;
    border-radius: var(--radius);
    color: var(--muted-foreground);
    transition: background 0.15s, color 0.15s;
  }

  .theme-toggle:hover {
    background: var(--accent-hover);
    color: var(--fg);
  }
</style>
