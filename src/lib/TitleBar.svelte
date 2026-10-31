<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { X } from 'lucide-svelte';
  import { app, activeVault, closeAllDialogs } from './stores.svelte';
  import { requestQuit, titleParts } from './window-chrome';

  // The window has no native frame (decorations: false). This bar is the drag
  // area: `data-tauri-drag-region` is not inherited, so every non-interactive
  // element that should drag carries it. Edge resizing is handled natively by
  // the Tauri runtime for undecorated windows (see frameless-desktop-window D4).
  let pageTitle = $derived(app.pages.find((p) => p.uuid === app.currentUuid)?.title);
  let parts = $derived(titleParts(activeVault()?.name, pageTitle));

  // The window outline in app.css applies only to a restored window;
  // `window-maximized` on <html> removes it.
  onMount(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    const win = getCurrentWindow();
    const sync = async () => {
      const full = (await win.isMaximized()) || (await win.isFullscreen());
      document.documentElement.classList.toggle('window-maximized', full);
    };
    sync().catch(() => {
      // Not running in Tauri: keep the restored look.
    });
    win
      .onResized(() => void sync().catch(() => {}))
      .then((fn) => (disposed ? fn() : (unlisten = fn)))
      .catch(() => {});
    return () => {
      disposed = true;
      unlisten?.();
    };
  });
</script>

<header class="titlebar" data-tauri-drag-region>
  <div class="titlebar-title" data-tauri-drag-region>
    {#if parts.length}
      {#each parts as part, i (i)}
        {#if i > 0}<span class="titlebar-sep" data-tauri-drag-region>·</span>{/if}
        <span class={i === 0 ? 'titlebar-vault' : 'titlebar-page'} data-tauri-drag-region>{part}</span>
      {/each}
    {:else}
      <span class="titlebar-vault" data-tauri-drag-region>Cerbo</span>
    {/if}
  </div>
  <button
    class="titlebar-close"
    onclick={() => requestQuit(app, closeAllDialogs)}
    title="Quit Cerbo"
    aria-label="Quit Cerbo"
  >
    <X size={16} />
  </button>
</header>

<style>
  .titlebar {
    display: flex;
    align-items: center;
    height: var(--titlebar-height);
    flex-shrink: 0;
    padding-left: 0.75rem;
    background: var(--sidebar-bg);
    border-bottom: 1px solid var(--border);
    user-select: none;
    -webkit-user-select: none;
  }

  .titlebar-title {
    flex: 1;
    min-width: 0;
    height: 100%;
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.8rem;
    color: var(--muted-foreground);
    overflow: hidden;
    white-space: nowrap;
  }

  .titlebar-vault,
  .titlebar-page {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .titlebar-vault {
    flex-shrink: 0;
    max-width: 50%;
    font-weight: 600;
    color: var(--fg);
  }

  .titlebar-sep {
    flex-shrink: 0;
  }

  .titlebar-close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 100%;
    flex-shrink: 0;
    padding: 0;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--muted-foreground);
    transition: background 0.15s, color 0.15s;
  }

  .titlebar-close:hover,
  .titlebar-close:focus-visible {
    background: #ef4444;
    color: #fff;
    outline: none;
  }
</style>
