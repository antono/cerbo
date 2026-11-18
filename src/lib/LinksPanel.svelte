<script lang="ts">
  /**
   * The Links sub-panel: Backlinks, Outgoing links and Similar notes of the
   * open page (spec `desktop-right-panel`, design D2). It stays mounted while
   * another tab is selected and loads nothing then (`visible`, design D3).
   */
  import { untrack } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { app, loadBacklinks, loadOutgoingLinks } from './stores.svelte';
  import { INDEX_READY_EVENT } from './backlinks';
  import LinkListSection from './LinkListSection.svelte';
  import SimilarNotesSection from './SimilarNotesSection.svelte';

  let { uuid, visible = true }: { uuid: string; visible?: boolean } = $props();

  // Load both lists when the page changes or the tab is selected. The loaders
  // read and write `app`, so only `uuid` and `visible` are dependencies.
  $effect(() => {
    const current = uuid;
    if (current && visible) {
      untrack(() => {
        void loadBacklinks(current);
        void loadOutgoingLinks(current);
      });
    }
  });

  // A vault's index may still be building when its page opens; load again
  // once the desktop reports it ready.
  $effect(() => {
    const current = uuid;
    const unlisten = listen(INDEX_READY_EVENT, () => {
      if (!current || !visible) return;
      void loadBacklinks(current);
      void loadOutgoingLinks(current);
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  });
</script>

<div class="links-panel" data-testid="links-panel">
  <div class="panel-content">
    <LinkListSection
      id="backlinks"
      title="Backlinks"
      entries={app.backlinks}
      status={app.backlinksStatus}
      message={app.backlinksMessage}
      loadingText="Loading backlinks…"
      emptyText="No pages link here."
    />
    <LinkListSection
      id="outgoing"
      title="Outgoing links"
      entries={app.outgoingLinks}
      status={app.outgoingStatus}
      message={app.outgoingMessage}
      loadingText="Loading outgoing links…"
      emptyText="This page links to no pages."
    />
    <SimilarNotesSection {uuid} {visible} />
  </div>
</div>

<style>
  .links-panel {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .panel-content {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  /* The sections space themselves with the gap above. */
  .panel-content :global(.similar-section) {
    margin-top: 0;
  }
</style>
