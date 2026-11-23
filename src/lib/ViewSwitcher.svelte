<script lang="ts">
  /**
   * The sidebar's view selector (spec `desktop-view-switcher`): "All notes",
   * the built-in default, then the vault's views by title. The choice is
   * remembered per vault by the store. A native `<select>` keeps it usable by
   * touch and keyboard without extra code.
   */
  import { app, loadViews, setActiveView } from './stores.svelte';

  /** The list is refreshed when the selector is used: an agent may have added or removed a view. */
  async function refresh() {
    await loadViews();
    if (app.activeViewId && !app.views.some((v) => v.uuid === app.activeViewId)) await setActiveView(null);
  }

  function onChange(e: Event) {
    const value = (e.currentTarget as HTMLSelectElement).value;
    void setActiveView(value === '' ? null : value);
  }
</script>

<div class="view-switcher" data-testid="view-switcher">
  <select
    aria-label="Sidebar view"
    title="Sidebar view"
    value={app.activeViewId ?? ''}
    onchange={onChange}
    onfocus={refresh}
    onmousedown={refresh}
  >
    <option value="">All notes</option>
    {#each app.views as view (view.uuid)}
      <option value={view.uuid}>{view.title}</option>
    {/each}
  </select>
  {#if app.viewError}
    <p class="view-error" data-testid="view-error" title={app.viewError}>This view could not be loaded.</p>
  {/if}
</div>

<style>
  .view-switcher {
    flex-shrink: 0;
    padding: 0.375rem 0.375rem 0.125rem;
  }
  select {
    width: 100%;
    height: 28px;
    padding: 0 0.4rem;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
    color: var(--fg);
    font: inherit;
    font-size: 0.8125rem;
    cursor: pointer;
  }
  select:focus-visible {
    outline: none;
    box-shadow: inset 0 0 0 1px var(--primary);
  }
  .view-error {
    margin: 0.25rem 0 0;
    font-size: 0.75rem;
    color: #dc2626;
  }
  @media (pointer: coarse) {
    select {
      height: 44px;
      font-size: 0.9375rem;
    }
  }
</style>
