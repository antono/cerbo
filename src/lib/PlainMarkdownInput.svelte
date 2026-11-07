<script lang="ts">
  /**
   * PlainMarkdownInput — write-mode editor for large pages.
   *
   * Carta's own `Input` diffs the whole document and rewrites its shiki overlay
   * on every key, which costs a frame or more above `LARGE_DOCUMENT_CHARS`
   * (see editor-performance.ts). This component keeps Carta's DOM classes,
   * toolbar icons, keyboard shortcuts, history and input extensions (link
   * autocomplete, emoji, attachments) through `carta.$setInput`, but shows a
   * plain textarea that scrolls itself: no overlay (so no syntax colours), no
   * renderer, no resize. Caret positions handed to popups are corrected for the
   * textarea's own scroll offset.
   */
  import type { Carta } from 'carta-md';
  import { onMount } from 'svelte';
  import { offsetCaretByTextareaScroll } from './editor-performance';

  let {
    carta,
    value = $bindable(''),
    theme = 'default',
  }: {
    carta: Carta;
    value?: string;
    theme?: string;
  } = $props();

  let editorElem: HTMLDivElement;
  let inputElem: HTMLDivElement;
  let textarea: HTMLTextAreaElement;
  let mounted = $state(false);

  const inputComponents = $derived(carta.components.filter(({ parent }) => [parent].flat().includes('input')));
  const editorComponents = $derived(carta.components.filter(({ parent }) => [parent].flat().includes('editor')));

  // Carta re-places caret popups on window scroll, but this textarea scrolls
  // itself (e.g. to keep the caret visible after a key). Forward it, once a frame.
  let scrollFrame = 0;
  function forwardScroll() {
    if (scrollFrame) return;
    scrollFrame = requestAnimationFrame(() => {
      scrollFrame = 0;
      window.dispatchEvent(new Event('scroll'));
    });
  }

  onMount(() => {
    carta.$setElement(editorElem);
    carta.$setInput(textarea, inputElem, () => {
      value = textarea.value;
    });
    if (carta.input) offsetCaretByTextareaScroll(carta.input);
    mounted = true;
    return () => cancelAnimationFrame(scrollFrame);
  });
</script>

<div bind:this={editorElem} class="carta-editor carta-theme__{theme}" data-input-mode="plain">
  <div class="carta-toolbar" role="toolbar">
    <div class="carta-toolbar-left"></div>
    <div class="carta-filler"></div>
    <div class="carta-toolbar-right">
      {#each carta.icons as icon, index (icon.id)}
        <button
          type="button"
          class="carta-icon"
          tabindex={index == 0 ? 0 : -1}
          title={icon.label}
          aria-label={icon.label}
          onclick={(e) => {
            e.stopPropagation();
            e.preventDefault();
            if (carta.input) icon.action(carta.input);
            carta.input?.update();
            carta.input?.textarea.focus();
          }}
        >
          <icon.component />
        </button>
      {/each}
    </div>
  </div>

  <div class="carta-wrapper">
    <div class="carta-container mode-tabs">
      <div class="carta-input carta-plain" bind:this={inputElem}>
        <textarea
          class="carta-font-code carta-plain-input"
          aria-multiline="true"
          spellcheck="false"
          bind:value
          bind:this={textarea}
          onscroll={forwardScroll}
        ></textarea>
        {#if mounted}
          {#each inputComponents as { component: DynamicComponent, props }, index (index)}
            <DynamicComponent {carta} {...props}></DynamicComponent>
          {/each}
        {/if}
      </div>
    </div>
  </div>

  {#if mounted}
    {#each editorComponents as { component: DynamicComponent, props }, index (index)}
      <DynamicComponent {carta} {...props}></DynamicComponent>
    {/each}
  {/if}
</div>

<style>
  .carta-editor {
    position: relative;
    display: flex;
    flex-direction: column;
  }

  .carta-toolbar {
    height: 2rem;
    display: flex;
    flex-shrink: 0;
    overflow-x: auto;
    overflow-y: hidden;
  }

  .carta-filler {
    flex: 1;
  }

  .carta-toolbar-right {
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: flex-end;
  }

  .carta-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.5rem;
    height: 1.5rem;
    border-radius: 3px;
    cursor: pointer;
    margin-left: 4px;
  }

  .carta-container {
    display: flex;
    position: relative;
    height: 100%;
  }

  /* The textarea is the scroller here; the wrapper only pads it. */
  .carta-input.carta-plain {
    position: relative;
    width: 100%;
    display: flex;
    overflow: hidden !important;
  }

  /* Beats app.css, which hides textarea text for Carta's overlay. */
  .carta-input textarea.carta-plain-input {
    flex: 1;
    width: 100%;
    height: 100%;
    padding: 0;
    margin: 0;
    border: 0;
    outline: none;
    background: transparent;
    tab-size: 4;
    white-space: pre-wrap;
    overflow-wrap: break-word;
    -webkit-text-fill-color: currentColor !important;
    overflow-y: auto !important;
    resize: none !important;
  }
</style>
