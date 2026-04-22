<script lang="ts">
  import { mode, setMode } from 'mode-watcher';
  import { Sun, Moon } from 'lucide-svelte';

  function toggleTheme() {
    if (mode.current === 'light') {
      setMode('dark');
    } else {
      setMode('light');
    }
  }
</script>

<button
  class="theme-toggle"
  onclick={toggleTheme}
  title="Toggle theme"
  aria-label="Toggle theme"
>
  {#if mode.current === 'dark'}
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
