<script lang="ts">
  /** A small context menu for a sidebar row (design D8). */
  import type { Pencil } from 'lucide-svelte';
  import { app } from './stores.svelte';

  export interface RowMenuItem {
    label: string;
    /** Any lucide icon; they all share one component type. */
    icon?: typeof Pencil;
    danger?: boolean;
    /** Shown but not selectable, e.g. "Move up" on the first row. */
    disabled?: boolean;
    /** Hotkey shown at the right edge, e.g. `Ctrl+T`. */
    hint?: string;
    /** Stable id for tests (`data-action`), e.g. the registry action id. */
    id?: string;
    onSelect: () => void;
  }

  let {
    x,
    y,
    items,
    onClose,
    label,
  }: { x: number; y: number; items: RowMenuItem[]; onClose: () => void; label?: string } = $props();

  let menuEl = $state<HTMLDivElement | null>(null);
  let left = $state(0);
  let top = $state(0);
  let positioned = $state(false);
  let highlighted = $state(0);

  $effect(() => {
    // Keep the menu inside the window.
    const el = menuEl;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    left = Math.max(4, Math.min(x, window.innerWidth - rect.width - 4));
    top = Math.max(4, Math.min(y, window.innerHeight - rect.height - 4));
    positioned = true;
    (el.querySelector('button:not(:disabled)') as HTMLButtonElement | null)?.focus();
  });

  $effect(() => {
    function onPointerDown(e: PointerEvent) {
      if (menuEl && !menuEl.contains(e.target as Node)) onClose();
    }
    function onKeydown(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        onClose();
      } else if (e.key === 'ArrowDown' || e.key === 'j') {
        // The menu owns these keys: stop them before the tree or the preview
        // behind the menu moves too.
        e.preventDefault();
        e.stopPropagation();
        step(1);
      } else if (e.key === 'ArrowUp' || e.key === 'k') {
        e.preventDefault();
        e.stopPropagation();
        step(-1);
      }
    }
    // Arrow keys skip disabled entries.
    function step(delta: 1 | -1) {
      for (let n = 1; n <= items.length; n++) {
        const i = (highlighted + delta * n + items.length * n) % items.length;
        if (!items[i].disabled) {
          highlighted = i;
          (menuEl?.querySelectorAll('button')[i] as HTMLButtonElement | undefined)?.focus();
          return;
        }
      }
    }
    window.addEventListener('pointerdown', onPointerDown, true);
    window.addEventListener('keydown', onKeydown, true);
    window.addEventListener('blur', onClose);
    window.addEventListener('resize', onClose);
    return () => {
      window.removeEventListener('pointerdown', onPointerDown, true);
      window.removeEventListener('keydown', onKeydown, true);
      window.removeEventListener('blur', onClose);
      window.removeEventListener('resize', onClose);
    };
  });

  function select(item: RowMenuItem) {
    onClose();
    item.onSelect();
  }
</script>

<!-- Narrow layout: menu targets are at least 32px tall (spec
     `desktop-responsive-layout`, "Narrow layout"). -->
<div class="row-menu" class:touch={app.widthClass === 'narrow'} bind:this={menuEl} style="left: {left}px; top: {top}px; visibility: {positioned ? 'visible' : 'hidden'};" role="menu" aria-label={label}>
  {#each items as item, i (item.label)}
    <button
      type="button"
      class="menu-item"
      class:danger={item.danger}
      role="menuitem"
      disabled={item.disabled}
      data-action={item.id}
      onclick={() => select(item)}
      onfocus={() => (highlighted = i)}
    >
      {#if item.icon}
        <item.icon size={14} />
      {/if}
      <span class="menu-label">{item.label}</span>
      {#if item.hint}
        <kbd class="menu-hint">{item.hint}</kbd>
      {/if}
    </button>
  {/each}
</div>

<style>
  .row-menu {
    position: fixed;
    z-index: 900;
    min-width: 11rem;
    padding: 0.25rem;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
    display: flex;
    flex-direction: column;
  }
  .menu-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.4rem 0.6rem;
    border: none;
    border-radius: 0.25rem;
    background: transparent;
    color: var(--fg);
    font-size: 0.85rem;
    text-align: left;
    cursor: pointer;
    outline: none;
  }
  .row-menu.touch .menu-item {
    min-height: 32px;
  }
  .menu-label {
    flex: 1;
  }
  .menu-hint {
    margin-left: 1.5rem;
    font-family: inherit;
    font-size: 0.75rem;
    color: var(--muted-foreground);
  }
  .menu-item:hover,
  .menu-item:focus {
    background: var(--accent-hover);
  }
  .menu-item.danger:hover,
  .menu-item.danger:focus {
    color: #dc2626;
  }
  .menu-item:disabled {
    opacity: 0.45;
    cursor: default;
    background: transparent;
  }
</style>
