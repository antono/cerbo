<script lang="ts">
  import { mode, setMode } from 'mode-watcher';

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
    <!-- Moon icon -->
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z" />
    </svg>
  {:else}
    <!-- Sun icon -->
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2v2" />
      <path d="M12 20v2" />
      <path d="m4.93 4.93 1.41 1.41" />
      <path d="m17.66 17.66 1.41 1.41" />
      <path d="M2 12h2" />
      <path d="M20 12h2" />
      <path d="m6.34 17.66-1.41 1.41" />
      <path d="m19.07 4.93-1.41 1.41" />
    </svg>
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
