<script lang="ts">
  /**
   * The composer: Enter sends, Shift+Enter breaks the line; the open page is a
   * removable context chip; `@` offers vault pages to attach; while the agent
   * works the send button becomes Cancel.
   */
  import { Send, Square, X } from 'lucide-svelte';
  import { app } from '../stores.svelte';
  import {
    applyMention,
    canSend,
    composerKey,
    detectMention,
    filterMentions,
    messageContext,
    type MentionTrigger,
  } from './composer';
  import { clampSelection, moveSelection } from '../link-picker';
  import type { PageRef } from './types';

  let {
    working,
    onSend,
    onCancel,
  }: {
    working: boolean;
    onSend: (text: string, context: PageRef[]) => boolean | Promise<boolean>;
    onCancel: () => void | Promise<void>;
  } = $props();

  let text = $state('');
  let input = $state<HTMLTextAreaElement | null>(null);
  let mentions = $state<PageRef[]>([]);
  /** The open page's chip was removed; the next page that opens brings it back. */
  let removedFor = $state<string | null>(null);
  let trigger = $state<MentionTrigger | null>(null);
  let selected = $state(0);
  let sending = $state(false);

  let openPage = $derived.by<PageRef | null>(() => {
    const uuid = app.currentUuid;
    if (!uuid) return null;
    const meta = app.pages.find((p) => p.uuid === uuid);
    return { path: uuid, title: meta?.title ?? uuid };
  });
  let openPageRemoved = $derived(openPage !== null && removedFor === openPage.path);
  let context = $derived(messageContext({ openPage, openPageRemoved, mentions }));

  let candidates = $derived(
    trigger
      ? filterMentions(
          app.pages,
          trigger.query,
          mentions.map((m) => m.path),
        )
      : [],
  );
  let popupOpen = $derived(trigger !== null && candidates.length > 0);

  $effect(() => {
    const clamped = clampSelection(selected, candidates.length);
    if (clamped !== selected) selected = clamped;
  });

  function updateTrigger() {
    if (!input) return;
    trigger = detectMention(text, input.selectionStart ?? text.length);
    selected = 0;
  }

  async function chooseMention(index: number) {
    const page = candidates[index];
    if (!page || !trigger || !input) return;
    const caret = input.selectionStart ?? text.length;
    const next = applyMention(text, trigger, caret);
    text = next.text;
    mentions = [...mentions, { path: page.uuid, title: page.title }];
    trigger = null;
    await Promise.resolve();
    input.focus();
    input.setSelectionRange(next.caret, next.caret);
  }

  function resize() {
    if (!input) return;
    input.style.height = 'auto';
    input.style.height = `${Math.min(input.scrollHeight, 160)}px`;
  }

  async function send() {
    if (!canSend(text, working) || sending) return;
    sending = true;
    const sent = text;
    const ctx = context;
    try {
      const ok = await onSend(sent, ctx);
      if (ok) {
        text = '';
        mentions = [];
        trigger = null;
        queueMicrotask(resize);
      }
    } finally {
      sending = false;
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (popupOpen) {
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        selected = moveSelection(selected, e.key === 'ArrowDown' ? 1 : -1, candidates.length);
        return;
      }
      if ((e.key === 'Enter' || e.key === 'Tab') && !e.isComposing) {
        e.preventDefault();
        void chooseMention(selected);
        return;
      }
      if (e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        trigger = null;
        return;
      }
    }
    if (composerKey(e) === 'send') {
      e.preventDefault();
      void send();
    }
  }
</script>

<div class="composer" data-testid="agent-composer">
  {#if context.length > 0}
    <ul class="chips" aria-label="Context pages" data-testid="agent-context">
      {#if openPage && !openPageRemoved}
        <li class="chip" data-testid="agent-context-chip" data-page={openPage.path} data-kind="open">
          <span class="chip-label" title={openPage.title}>{openPage.title}</span>
          <button
            class="chip-remove"
            data-testid="agent-context-remove"
            aria-label="Remove {openPage.title} from the context"
            onclick={() => (removedFor = openPage.path)}
          >
            <X size={11} />
          </button>
        </li>
      {/if}
      {#each mentions as ref (ref.path)}
        <li class="chip" data-testid="agent-context-chip" data-page={ref.path} data-kind="mention">
          <span class="chip-label" title={ref.title}>{ref.title}</span>
          <button
            class="chip-remove"
            aria-label="Remove {ref.title} from the context"
            onclick={() => (mentions = mentions.filter((m) => m.path !== ref.path))}
          >
            <X size={11} />
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  <div class="row">
    <div class="field">
      {#if popupOpen}
        <ul class="mentions" role="listbox" aria-label="Pages to attach" data-testid="agent-mentions">
          {#each candidates as page, i (page.uuid)}
            <li role="option" aria-selected={i === selected}>
              <button
                class="mention"
                class:active={i === selected}
                data-testid="agent-mention-option"
                tabindex="-1"
                onmousedown={(e) => {
                  e.preventDefault();
                  void chooseMention(i);
                }}
              >
                <span class="m-title">{page.title}</span>
                {#if page.virtualPath}<span class="m-path">{page.virtualPath}</span>{/if}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
      <textarea
        bind:this={input}
        bind:value={text}
        rows="1"
        placeholder="Ask the agent… (@ adds a page)"
        aria-label="Message to the agent"
        data-testid="agent-input"
        oninput={() => {
          updateTrigger();
          resize();
        }}
        onkeydown={onKeydown}
        onkeyup={(e) => {
          if (e.key.startsWith('Arrow') && !popupOpen) updateTrigger();
        }}
        onclick={updateTrigger}
        onblur={() => (trigger = null)}
      ></textarea>
    </div>
    {#if working}
      <button class="action cancel" data-testid="agent-cancel" aria-label="Cancel the agent's turn" title="Cancel" onclick={() => onCancel()}>
        <Square size={14} />
      </button>
    {:else}
      <button
        class="action send"
        data-testid="agent-send"
        aria-label="Send"
        title="Send (Enter)"
        disabled={!canSend(text, working) || sending}
        onclick={send}
      >
        <Send size={14} />
      </button>
    {/if}
  </div>
</div>

<style>
  .composer {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    padding: 0.5rem 0.75rem 0.75rem;
    border-top: 1px solid var(--border);
  }

  .chips {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }

  .chip {
    display: flex;
    align-items: center;
    gap: 0.15rem;
    max-width: 100%;
    padding: 0.1rem 0.2rem 0.1rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--accent);
    font-size: 0.72rem;
  }

  .chip-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chip-remove {
    display: flex;
    padding: 2px;
    border: none;
    border-radius: 50%;
    background: none;
    color: var(--muted-foreground);
    cursor: pointer;
  }

  .chip-remove:hover,
  .chip-remove:focus-visible {
    background: var(--accent-hover);
    color: var(--fg);
    outline: none;
  }

  .row {
    display: flex;
    align-items: flex-end;
    gap: 0.4rem;
  }

  .field {
    position: relative;
    flex: 1;
    min-width: 0;
  }

  textarea {
    width: 100%;
    box-sizing: border-box;
    resize: none;
    padding: 0.4rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
    color: var(--fg);
    font: inherit;
    font-size: 0.8125rem;
    line-height: 1.4;
  }

  textarea:focus-visible {
    outline: 2px solid var(--primary);
    outline-offset: -1px;
  }

  .action {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    flex-shrink: 0;
    border: none;
    border-radius: var(--radius);
    background: var(--primary);
    color: var(--primary-foreground);
    cursor: pointer;
  }

  .action.cancel {
    background: #ef4444;
  }

  .action:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .action:focus-visible {
    outline: 2px solid var(--fg);
    outline-offset: 1px;
  }

  .mentions {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 100%;
    z-index: 10;
    margin: 0 0 4px;
    padding: 0.25rem;
    list-style: none;
    max-height: 12rem;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
    box-shadow: 0 4px 12px rgb(0 0 0 / 0.2);
  }

  .mention {
    display: flex;
    flex-direction: column;
    width: 100%;
    padding: 0.3rem 0.5rem;
    border: none;
    border-radius: var(--radius);
    background: none;
    color: var(--fg);
    font-size: 0.8125rem;
    text-align: left;
    cursor: pointer;
  }

  .mention.active,
  .mention:hover {
    background: var(--accent-hover);
  }

  .m-title,
  .m-path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .m-path {
    font-size: 0.7rem;
    color: var(--muted-foreground);
  }
</style>
