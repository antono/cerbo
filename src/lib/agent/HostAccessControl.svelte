<script lang="ts">
  /**
   * Host access of the vault (spec `agent-host-access`): may the agent work
   * with files outside the vault. Turning it on, and allowing writing, ask
   * first. While it is on, the panel says so and lists the allowed paths.
   */
  import { open } from '@tauri-apps/plugin-dialog';
  import { homeDir } from '@tauri-apps/api/path';
  import ConfirmationDialog from '../ConfirmationDialog.svelte';
  import { HOST_ON_WARNING, hostSummary, modeLabel, writeWarning, type HostAccess } from './host';

  let {
    host,
    warning = null,
    onEnable,
    onAdd,
    onRemove,
  }: {
    host: HostAccess;
    /** A note about the last path added, such as "covers your whole home folder". */
    warning?: string | null;
    onEnable: (enabled: boolean) => void | Promise<void>;
    onAdd: (path: string, write: boolean) => void | Promise<void>;
    onRemove: (path: string) => void | Promise<void>;
  } = $props();

  let expanded = $state(false);
  let path = $state('');
  let write = $state(false);
  let confirm = $state<'on' | 'write' | null>(null);

  let summary = $derived(hostSummary(host));

  function toggle() {
    if (host.enabled) void onEnable(false);
    else confirm = 'on';
  }

  async function browse() {
    const selected = await open({ directory: true, multiple: false, defaultPath: await homeDir() });
    if (selected && typeof selected === 'string') path = selected;
  }

  function add() {
    if (path.trim() === '') return;
    if (write) confirm = 'write';
    else void commitAdd();
  }

  async function commitAdd() {
    const value = path.trim();
    path = '';
    await onAdd(value, write);
    write = false;
  }

  async function confirmed() {
    const what = confirm;
    confirm = null;
    if (what === 'on') await onEnable(true);
    else if (what === 'write') await commitAdd();
  }
</script>

<div class="host" data-testid="agent-host" data-enabled={host.enabled}>
  <div class="row">
    <button
      class="head"
      aria-expanded={expanded}
      aria-controls="agent-host-body"
      data-testid="agent-host-toggle-details"
      onclick={() => (expanded = !expanded)}
    >
      <span class="label">Files outside the vault</span>
      {#if summary}
        <span class="marker" data-testid="agent-host-marker">{summary}</span>
      {:else}
        <span class="off">Off</span>
      {/if}
    </button>
    <button
      class="switch"
      role="switch"
      aria-checked={host.enabled}
      aria-label="Let the agent use files outside the vault"
      data-testid="agent-host-switch"
      onclick={toggle}
    >
      {host.enabled ? 'On' : 'Off'}
    </button>
  </div>

  {#if expanded || host.enabled}
    <div class="body" id="agent-host-body">
      {#if host.enabled}
        {#if host.paths.length === 0}
          <p class="note" data-testid="agent-host-empty">No path is open: the agent asks you for every file.</p>
        {:else}
          <ul class="paths" data-testid="agent-host-paths">
            {#each host.paths as p (p.path)}
              <li data-testid="agent-host-path" data-write={p.write}>
                <span class="path" title={p.path}>{p.path}</span>
                <span class="mode">{modeLabel(p)}</span>
                <button class="remove" aria-label="Stop allowing {p.path}" data-testid="agent-host-remove" onclick={() => onRemove(p.path)}>×</button>
              </li>
            {/each}
          </ul>
        {/if}
        <form class="add" onsubmit={(e) => { e.preventDefault(); add(); }}>
          <input
            type="text"
            placeholder="/path/to/folder"
            aria-label="Folder to allow"
            data-testid="agent-host-input"
            bind:value={path}
          />
          <button type="button" class="browse" data-testid="agent-host-browse" onclick={browse}>Browse…</button>
          <label class="write"><input type="checkbox" data-testid="agent-host-write" bind:checked={write} /> Allow writing</label>
          <button type="submit" class="submit" data-testid="agent-host-add" disabled={path.trim() === ''}>Allow</button>
        </form>
        {#if warning}
          <p class="warn" role="alert" data-testid="agent-host-warning">{warning}</p>
        {/if}
      {:else}
        <p class="note">Off: the agent can only reach this vault. Turn it on to let it read, and where you allow it write, files elsewhere.</p>
      {/if}
    </div>
  {/if}
</div>

{#if confirm}
  <div data-testid="agent-host-confirm">
    <ConfirmationDialog
      title={confirm === 'on' ? 'Let the agent use files outside the vault?' : 'Let the agent write files?'}
      message={confirm === 'on' ? HOST_ON_WARNING : writeWarning(path.trim())}
      confirmLabel="Allow"
      onConfirm={confirmed}
      onClose={() => (confirm = null)}
    />
  </div>
{/if}

<style>
  .host {
    border-bottom: 1px solid var(--border);
    font-size: 0.75rem;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.75rem;
  }

  .head {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0;
    border: none;
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .label {
    color: var(--muted-foreground);
    flex-shrink: 0;
  }

  .off {
    color: var(--muted-foreground);
  }

  .marker {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: #b45309;
    font-weight: 600;
  }

  .switch {
    flex-shrink: 0;
    min-width: 2.6rem;
    padding: 0.2rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
    color: var(--muted-foreground);
    font-size: 0.72rem;
    cursor: pointer;
  }

  .switch[aria-checked='true'] {
    background: var(--primary);
    color: var(--primary-foreground);
  }

  .body {
    padding: 0 0.75rem 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .note,
  .warn {
    margin: 0;
    color: var(--muted-foreground);
  }

  .warn {
    padding: 0.3rem 0.5rem;
    background: rgb(245 158 11 / 0.15);
    border: 1px solid #f59e0b;
    border-radius: var(--radius);
    color: var(--fg);
    overflow-wrap: anywhere;
  }

  .paths {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .paths li {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono, monospace);
  }

  .mode {
    flex-shrink: 0;
    color: var(--muted-foreground);
  }

  li[data-write='true'] .mode {
    color: #b45309;
  }

  .remove {
    flex-shrink: 0;
    border: none;
    background: none;
    color: var(--muted-foreground);
    cursor: pointer;
    font-size: 1rem;
    line-height: 1;
  }

  .remove:hover,
  .remove:focus-visible {
    color: var(--fg);
    outline: none;
  }

  .add {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4rem;
  }

  .add input[type='text'] {
    flex: 1 1 8rem;
    min-width: 0;
    padding: 0.2rem 0.4rem;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
    color: var(--fg);
    font: inherit;
  }

  .write {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    color: var(--muted-foreground);
  }

  .browse,
  .submit {
    padding: 0.2rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
    color: var(--fg);
    font-size: 0.72rem;
    cursor: pointer;
  }

  .submit:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
