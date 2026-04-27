<script lang="ts">
  import { onMount } from 'svelte';

  let {
    title,
    message,
    confirmLabel,
    onConfirm,
    onClose,
    confirmDisabled = false,
  }: {
    title: string;
    message: string;
    confirmLabel: string;
    onConfirm: () => void | Promise<void>;
    onClose: () => void;
    confirmDisabled?: boolean;
  } = $props();

  let selectedIndex = $state(0);
  let modalEl = $state<HTMLElement | null>(null);

  onMount(() => {
    modalEl?.focus();

    function handleWindowKeydown(e: KeyboardEvent) {
      e.stopPropagation();

      if (e.key === 'ArrowLeft' || e.key === 'ArrowRight' || e.key === 'ArrowUp' || e.key === 'ArrowDown') {
        e.preventDefault();
        selectedIndex = selectedIndex === 0 ? 1 : 0;
      } else if (e.key === 'Enter') {
        e.preventDefault();
        if (selectedIndex === 1 && !confirmDisabled) {
          void onConfirm();
        } else if (selectedIndex === 0) {
          onClose();
        }
      } else if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
      } else if (!['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) {
        e.preventDefault();
      }
    }

    window.addEventListener('keydown', handleWindowKeydown, true);
    return () => window.removeEventListener('keydown', handleWindowKeydown, true);
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="modal-backdrop" onclick={onClose} role="presentation">
  <div
    class="confirm-modal"
    onclick={(e) => e.stopPropagation()}
    role="dialog"
    aria-modal="true"
    tabindex="0"
    bind:this={modalEl}
  >
    <div class="modal-content">
      <div class="icon-wrap">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="32"
          height="32"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="3"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <line x1="12" y1="8" x2="12" y2="12"></line>
          <line x1="12" y1="16" x2="12.01" y2="16"></line>
        </svg>
      </div>

      <div class="text-wrap">
        <h3>{title}</h3>
        <p>{message}</p>
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
        onclick={onConfirm}
        disabled={confirmDisabled}
      >
        {confirmLabel}
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

  .confirm-modal {
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
