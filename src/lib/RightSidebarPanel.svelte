<script lang="ts">
  /**
   * The right panel's tab host (change `add-right-panel-tabs`, spec
   * `desktop-right-panel`, design D1-D3). A strip of icon tabs along the top
   * shows one sub-panel at a time. Every sub-panel stays mounted: the
   * unselected ones are hidden but keep their layout, so each keeps its scroll
   * position, and they get `visible={false}` so they load nothing.
   *
   * The docked panel, the medium floating panel and the narrow sheet all
   * render this component, so they share the strip and the selected tab.
   */
  import type { Component } from 'svelte';
  import { app, selectRightPanelTab } from './stores.svelte';
  import { badgeText, RIGHT_PANELS, tabBadge, tabForKey, type RightPanelId } from './right-panels';
  import { controlTitle } from './toolbar';
  import AgentPanel from './AgentPanel.svelte';
  import AttachmentsPanel from './AttachmentsPanel.svelte';
  import LinksPanel from './LinksPanel.svelte';
  import OutlinePanel from './OutlinePanel.svelte';

  let { uuid }: { uuid: string } = $props();

  // One component per registry entry; the registry itself stays free of Svelte.
  const COMPONENTS: Record<RightPanelId, Component<{ uuid: string; visible?: boolean }>> = {
    attachments: AttachmentsPanel,
    links: LinksPanel,
    outline: OutlinePanel,
    agent: AgentPanel as unknown as Component<{ uuid: string; visible?: boolean }>,
  };

  let selected = $derived(app.rightPanelTab);
  let tablistEl = $state<HTMLElement | null>(null);

  // WAI-ARIA tabs: ArrowLeft/ArrowRight (and Home/End) move between tabs with
  // a roving tabindex; the moved-to tab is selected and focused.
  function onTablistKeydown(e: KeyboardEvent) {
    const next = tabForKey(e.key, selected);
    if (!next) return;
    e.preventDefault();
    e.stopPropagation();
    selectRightPanelTab(next);
    tablistEl?.querySelector<HTMLElement>(`#right-tab-${next}`)?.focus();
  }
</script>

<div class="right-sidebar-panel" data-testid="right-sidebar-panel" data-selected-tab={selected}>
  <div
    class="tab-strip"
    role="tablist"
    aria-label="Right panel sections"
    tabindex="-1"
    data-testid="right-tabs"
    bind:this={tablistEl}
    onkeydown={onTablistKeydown}
  >
    {#each RIGHT_PANELS as panel (panel.id)}
      {@const badge = tabBadge(panel.id, { attachmentCount: app.attachments.length, agentStatus: app.agentStatus })}
      {@const isSelected = selected === panel.id}
      <button
        class="tab"
        role="tab"
        id="right-tab-{panel.id}"
        aria-controls="right-panel-{panel.id}"
        aria-selected={isSelected}
        aria-label={badge ? `${panel.label}, ${badge.label}` : panel.label}
        tabindex={isSelected ? 0 : -1}
        data-testid="right-tab-{panel.id}"
        title={controlTitle(panel.label, panel.hotkey)}
        onclick={() => selectRightPanelTab(panel.id)}
      >
        <panel.icon size={16} />
        {#if badge?.kind === 'count'}
          <span class="badge count" data-testid="right-tab-badge-{panel.id}" aria-hidden="true">{badgeText(badge.value)}</span>
        {:else if badge?.kind === 'status'}
          <span
            class="badge status"
            data-testid="right-tab-badge-{panel.id}"
            data-status={badge.status}
            aria-hidden="true"
          ></span>
        {/if}
      </button>
    {/each}
  </div>

  <div class="panel-stack">
    {#each RIGHT_PANELS as panel (panel.id)}
      {@const Panel = COMPONENTS[panel.id]}
      {@const isSelected = selected === panel.id}
      <div
        class="panel-body"
        role="tabpanel"
        id="right-panel-{panel.id}"
        aria-labelledby="right-tab-{panel.id}"
        data-testid="right-panel-{panel.id}"
        data-hidden={!isSelected}
        inert={!isSelected}
      >
        <Panel {uuid} visible={isSelected} />
      </div>
    {/each}
  </div>
</div>

<style>
  .right-sidebar-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--sidebar-bg);
    overflow: hidden;
  }

  /* Stays put while a sub-panel scrolls. */
  .tab-strip {
    display: flex;
    gap: 0.125rem;
    padding: 0.25rem 0.5rem;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .tab {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    background: none;
    border: none;
    border-radius: var(--radius);
    color: var(--muted-foreground);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .tab:hover,
  .tab:focus-visible {
    background: var(--accent-hover);
    color: var(--fg);
    outline: none;
  }

  .tab[aria-selected='true'] {
    background: var(--accent);
    color: var(--primary);
    box-shadow: inset 0 -2px 0 var(--primary);
  }

  .badge {
    position: absolute;
    pointer-events: none;
  }

  .badge.count {
    top: 1px;
    right: 0;
    min-width: 14px;
    height: 14px;
    padding: 0 3px;
    box-sizing: border-box;
    border-radius: 7px;
    background: var(--primary);
    color: var(--primary-foreground);
    font-size: 0.625rem;
    font-weight: 700;
    line-height: 14px;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }

  /* Agent status mark: hollow when not connected, filled when connected. */
  .badge.status {
    top: 4px;
    right: 4px;
    width: 7px;
    height: 7px;
    box-sizing: border-box;
    border-radius: 50%;
    border: 1.5px solid var(--muted-foreground);
    background: var(--sidebar-bg);
  }

  .badge.status[data-status='connected'] {
    background: var(--primary);
    border-color: var(--primary);
  }

  .badge.status[data-status='working'] {
    background: var(--primary);
    border-color: var(--primary);
    animation: pulse 1.2s ease-in-out infinite;
  }

  .badge.status[data-status='approval'] {
    background: #f59e0b;
    border-color: #f59e0b;
  }

  @keyframes pulse {
    50% {
      opacity: 0.4;
    }
  }

  .panel-stack {
    position: relative;
    flex: 1;
    min-height: 0;
  }

  /* Every sub-panel fills the stack. The hidden ones keep their layout (so
     their scroll position survives) and leave the screen and the tab order. */
  .panel-body {
    position: absolute;
    inset: 0;
    overflow: hidden;
  }

  .panel-body[data-hidden='true'] {
    visibility: hidden;
    pointer-events: none;
  }
</style>
