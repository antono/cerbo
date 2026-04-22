<script lang="ts">
  import { onMount } from 'svelte';
  import { AlertTriangle } from 'lucide-svelte';
  import { quitApp } from './stores.svelte';

  let { onClose }: { onClose: () => void } = $props();

  let selectedIndex = $state(0); // 0 = Cancel, 1 = Quit

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowLeft' || e.key === 'ArrowRight' || e.key === 'Tab') {
      e.preventDefault();
      selectedIndex = selectedIndex === 0 ? 1 : 0;
    } else if (e.key === 'Enter') {
      if (selectedIndex === 1) {
        quitApp();
      } else {
        onClose();
      }
    } else if (e.key === 'Escape') {
      onClose();
    }
  }

  onMount(() => {
    // We want the modal container or a button to catch initial focus if possible, 
    // but the window listener in layout will handle global keys.
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="modal-backdrop" onclick={onClose} role="presentation">
  <div 
    class="exit-modal" 
    onclick={(e) => e.stopPropagation()} 
    role="dialog" 
    aria-modal="true"
    onkeydown={handleKeydown}
    tabindex="-1"
  >
    <div class="modal-content">
      <div class="icon-wrap">
        <AlertTriangle size={32} class="warning-icon" />
      </div>
      <div class="text-wrap">
        <h3>Quit Application?</h3>
        <p>Are you sure you want to exit Cerbo?</p>
      </div>
    </div>

    <div class="modal-actions">
      <button 
        class="btn btn-ghost" 
        class:selected={selectedIndex === 0}
        onclick={onClose}
      >
        Cancel
      </button>
      <button 
        class="btn btn-danger" 
        class:selected={selectedIndex === 1}
        onclick={quitApp}
      >
        Quit
      </button>
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    backdrop-filter: blur(2px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .exit-modal {
    width: 100%;
    max-width: 400px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg, 12px);
    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.2);
    display: flex;
    flex-direction: column;
    outline: none;
  }

  .modal-content {
    display: flex;
    padding: 1.5rem;
    gap: 1.25rem;
  }

  .icon-wrap {
    background: #fee2e2;
    color: #dc2626;
    width: 3.5rem;
    height: 3.5rem;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .text-wrap h3 {
    margin: 0 0 0.5rem;
    font-size: 1.25rem;
    font-weight: 700;
  }

  .text-wrap p {
    margin: 0;
    color: var(--muted-foreground);
    font-size: 0.95rem;
    line-height: 1.5;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    padding: 1rem 1.5rem;
    background: var(--sidebar-bg);
    border-top: 1px solid var(--border);
  }

  .btn {
    padding: 0.5rem 1.25rem;
    border-radius: var(--radius);
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
    border: 1px solid transparent;
  }

  .btn-ghost {
    background: transparent;
    border-color: var(--border);
    color: var(--fg);
  }

  .btn-danger {
    background: #dc2626;
    color: #fff;
  }

  .btn.selected {
    outline: 2px solid var(--primary);
    outline-offset: 2px;
    box-shadow: 0 0 0 4px rgba(var(--primary-rgb), 0.2);
  }

  .btn-danger.selected {
     background: #b91c1c;
  }

  .btn-ghost.selected {
     background: var(--accent);
  }
</style>
