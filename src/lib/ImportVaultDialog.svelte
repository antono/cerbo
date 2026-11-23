<script lang="ts">
  /** "Import vault" (spec `vault-import-ui`): source app → folder → target → preview. */
  import { onMount } from 'svelte';
  import { Import } from 'lucide-svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { homeDir } from '@tauri-apps/api/path';
  import { listen } from '@tauri-apps/api/event';
  import { app, loadVaults, openVault, closeAllDialogs } from './stores.svelte';
  import {
    SOURCE_FORMATS, defaultVaultName, formatBytes, formatMismatch, plural, progressFraction, progressLabel,
    type Detection, type ImportProgress, type ImportReport, type SourceFormat,
  } from './import-vault';

  let { onClose }: { onClose: () => void } = $props();

  type Step = 'source' | 'folder' | 'target' | 'preview';
  const STEPS: { id: Step; label: string }[] = [
    { id: 'source', label: 'Source' },
    { id: 'folder', label: 'Folder' },
    { id: 'target', label: 'Target' },
    { id: 'preview', label: 'Preview' },
  ];

  let step = $state<Step>('source');
  let format = $state<SourceFormat>('obsidian');
  let source = $state('');
  let detected = $state<Detection | null>(null);
  let targetKind = $state<'new' | 'existing'>('new');
  let newPath = $state('');
  let newName = $state('');
  let existingId = $state('');
  let report = $state<ImportReport | null>(null);
  let force = $state(false);
  let busy = $state(false);
  let progress = $state<ImportProgress | null>(null);
  let error = $state('');

  const mismatch = $derived(formatMismatch(format, detected));
  const stepIndex = $derived(STEPS.findIndex((s) => s.id === step));
  const canNext = $derived(
    !busy &&
      (step === 'source' ||
        (step === 'folder' && source !== '' && !mismatch) ||
        (step === 'target' &&
          (targetKind === 'new' ? newPath.trim() !== '' && newName.trim() !== '' : existingId !== ''))),
  );

  async function pickFolder() {
    const selected = await open({ directory: true, multiple: false, defaultPath: await homeDir() });
    if (!selected || typeof selected !== 'string') return;
    source = selected;
    error = '';
    detected = null;
    if (!newName) newName = defaultVaultName(selected);
    try {
      detected = await invoke<Detection | null>('vault_import_detect', { source });
    } catch (e) {
      error = String(e);
    }
  }

  async function pickNewFolder() {
    const selected = await open({ directory: true, multiple: false, defaultPath: await homeDir() });
    if (selected && typeof selected === 'string') newPath = selected;
  }

  async function loadPreview() {
    busy = true;
    error = '';
    try {
      report = await invoke<ImportReport>('vault_import_plan', { source, format });
      step = 'preview';
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function runImport() {
    if (busy) return;
    busy = true;
    error = '';
    progress = null;
    const unlisten = await listen<ImportProgress>('vault-import-progress', (e) => {
      progress = e.payload;
    });
    try {
      const target =
        targetKind === 'new'
          ? { new: { path: newPath.trim(), name: newName.trim() } }
          : { existing: { id: existingId } };
      const done = await invoke<ImportReport>('vault_import_run', { source, format, target, force });
      await loadVaults();
      const id = done.vault?.id;
      onClose();
      if (id) await openVault(id);
    } catch (e) {
      error = String(e);
      busy = false;
    } finally {
      unlisten();
    }
  }

  function next() {
    if (!canNext) return;
    error = '';
    if (step === 'source') step = 'folder';
    else if (step === 'folder') step = 'target';
    else if (step === 'target') void loadPreview();
  }

  function back() {
    if (busy) return;
    error = '';
    if (step === 'preview') step = 'target';
    else if (step === 'target') step = 'folder';
    else if (step === 'folder') step = 'source';
  }

  onMount(() => {
    existingId = app.activeVaultId ?? app.vaults[0]?.id ?? '';
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        e.preventDefault();
        if (!busy) onClose();
      } else if (e.key === 'Enter' && !(e.target instanceof HTMLButtonElement || e.target instanceof HTMLInputElement && e.target.type === 'checkbox')) {
        e.preventDefault();
        if (step === 'preview') void runImport();
        else next();
      }
    }
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="modal-backdrop" onclick={() => !busy && onClose()} role="presentation">
  <div class="dialog-modal" data-testid="import-vault-dialog" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" tabindex="-1">
    <div class="header">
      <Import size={18} class="icon" />
      <span class="title">Import vault</span>
      <ol class="steps">
        {#each STEPS as s, i (s.id)}
          <li class:active={i === stepIndex} class:done={i < stepIndex}>{i + 1} {s.label}</li>
        {/each}
      </ol>
    </div>

    <div class="body">
      {#if step === 'source'}
        <div class="cards">
          {#each SOURCE_FORMATS as f (f.id)}
            <button type="button" class="card" class:selected={format === f.id} data-testid="import-source-{f.id}" onclick={() => { format = f.id; }}>
              <span class="card-title">{f.label}</span>
              <span class="hint">{f.hint}</span>
            </button>
          {/each}
        </div>
        <div class="hint">Your notes are only read. The source folder is never changed.</div>
      {:else if step === 'folder'}
        <div class="field">
          <span class="field-label">{SOURCE_FORMATS.find((f) => f.id === format)?.label} folder</span>
          <div class="row">
            <input class="text" readonly value={source} placeholder="No folder chosen" data-testid="import-source-path" />
            <button type="button" class="secondary-btn" onclick={pickFolder}>Choose…</button>
          </div>
          {#if source && detected && !mismatch}
            <div class="ok">Detected {detected.format === 'obsidian' ? 'Obsidian' : 'Tolaria'} ({detected.reason}).</div>
          {/if}
          {#if mismatch}
            <div class="warn" data-testid="import-mismatch">
              This looks like a {mismatch === 'obsidian' ? 'Obsidian' : 'Tolaria'} vault.
              <button type="button" class="link-btn" onclick={() => { format = mismatch; }}>Switch to {mismatch === 'obsidian' ? 'Obsidian' : 'Tolaria'}</button>
            </div>
          {/if}
        </div>
      {:else if step === 'target'}
        <div class="field">
          <label class="radio"><input type="radio" bind:group={targetKind} value="new" /> Create a new vault</label>
          {#if targetKind === 'new'}
            <div class="row">
              <input class="text" readonly value={newPath} placeholder="Empty folder for the vault" />
              <button type="button" class="secondary-btn" onclick={pickNewFolder}>Choose…</button>
            </div>
            <input class="text" bind:value={newName} placeholder="Vault name" aria-label="Vault name" />
          {/if}
          {#if app.vaults.length > 0}
            <label class="radio"><input type="radio" bind:group={targetKind} value="existing" /> Add to an existing vault</label>
            {#if targetKind === 'existing'}
              <select class="text" bind:value={existingId} aria-label="Existing vault">
                {#each app.vaults as v (v.id)}
                  <option value={v.id}>{v.name} ({v.path})</option>
                {/each}
              </select>
            {/if}
          {/if}
        </div>
      {:else if report}
        <div class="summary" data-testid="import-preview">
          <strong>{plural(report.pages, 'page')}</strong> and
          <strong>{plural(report.attachments, 'attachment')}</strong> ({formatBytes(report.attachmentBytes)}),
          {plural(report.links, 'link')} rewritten, {plural(report.relations, 'relation')}.
        </div>
        {#if report.unresolved.length > 0}
          <details><summary>{plural(report.unresolved.length, 'unresolved link')} stay as text</summary>
            <ul>{#each report.unresolved.slice(0, 50) as u}<li>{u.note}: {u.link}</li>{/each}</ul>
          </details>
        {/if}
        {#if report.skipped.length > 0}
          <details><summary>{report.skipped.length} skipped</summary>
            <ul>{#each report.skipped.slice(0, 50) as s}<li>{s}</li>{/each}</ul>
          </details>
        {/if}
        {#if report.unreferenced.length > 0}
          <details><summary>{report.unreferenced.length} unreferenced files, not imported</summary>
            <ul>{#each report.unreferenced.slice(0, 50) as s}<li>{s}</li>{/each}</ul>
          </details>
        {/if}
        <label class="radio"><input type="checkbox" bind:checked={force} /> Import again anyway (makes a second copy)</label>
      {/if}

      {#if step === 'preview' && busy}
        <div class="progress" data-testid="import-progress">
          <progress max="1" value={progressFraction(progress)} aria-label="Import progress"></progress>
          <span class="hint">{progressLabel(progress)}</span>
        </div>
      {/if}

      {#if error}
        <div class="error-msg" data-testid="import-error">{error}</div>
      {/if}
    </div>

    <div class="dialog-footer">
      <div class="shortcuts">
        <span><kbd>↵</kbd> {step === 'preview' ? 'to import' : 'next'}</span>
        <span><kbd>esc</kbd> to close</span>
      </div>
      <div class="actions">
        {#if step !== 'source'}
          <button type="button" class="secondary-btn" onclick={back} disabled={busy}>Back</button>
        {/if}
        {#if step === 'preview'}
          <button type="button" class="primary-btn" onclick={runImport} disabled={busy} data-testid="import-run">
            {busy ? 'Importing…' : 'Import'}
          </button>
        {:else}
          <button type="button" class="primary-btn" onclick={next} disabled={!canNext}>
            {busy ? 'Reading…' : 'Next'}
          </button>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .modal-backdrop { position: fixed; inset: 0; background: rgba(0,0,0,.4); backdrop-filter: blur(2px); display: flex; align-items: flex-start; justify-content: center; padding-top: 15vh; z-index: 1000; }
  .dialog-modal { width: 100%; max-width: 560px; background: var(--bg); border: 1px solid var(--border); border-radius: var(--radius-lg, 12px); box-shadow: 0 10px 40px rgba(0,0,0,.2); display: flex; flex-direction: column; }
  .header { display: flex; align-items: center; padding: .75rem 1rem; border-bottom: 1px solid var(--border); gap: .75rem; background: var(--sidebar-bg); border-radius: var(--radius-lg, 12px) var(--radius-lg, 12px) 0 0; }
  .title { font-size: .9rem; font-weight: 600; color: var(--muted-foreground); text-transform: uppercase; letter-spacing: .05em; }
  .steps { display: flex; gap: .75rem; margin: 0 0 0 auto; padding: 0; list-style: none; font-size: .75rem; color: var(--muted-foreground); }
  .steps .active { color: var(--foreground, inherit); font-weight: 600; }
  .steps .done { opacity: .7; }
  .body { padding: 1.25rem 1.5rem; display: flex; flex-direction: column; gap: 1rem; max-height: 60vh; overflow: auto; }
  .cards { display: grid; grid-template-columns: 1fr 1fr; gap: .75rem; }
  .card { display: flex; flex-direction: column; gap: .25rem; text-align: left; padding: .9rem 1rem; border: 1px solid var(--border); border-radius: var(--radius); background: var(--bg); cursor: pointer; color: inherit; }
  .card.selected { border-color: var(--primary); box-shadow: 0 0 0 1px var(--primary); }
  .card-title { font-weight: 600; }
  .field { display: flex; flex-direction: column; gap: .6rem; }
  .field-label { font-size: .8rem; font-weight: 600; color: var(--muted-foreground); text-transform: uppercase; }
  .row { display: flex; gap: .5rem; }
  .text { flex: 1; padding: .4rem .6rem; border: 1px solid var(--border); border-radius: var(--radius); background: var(--bg); color: inherit; font: inherit; font-size: .85rem; }
  .radio { display: flex; align-items: center; gap: .5rem; font-size: .85rem; }
  .hint { font-size: .8rem; color: var(--muted-foreground); }
  .ok { font-size: .8rem; color: var(--muted-foreground); }
  .warn { font-size: .85rem; color: #d97706; }
  .link-btn { background: none; border: none; color: var(--primary); cursor: pointer; padding: 0; font: inherit; text-decoration: underline; }
  .progress { display: flex; flex-direction: column; gap: .35rem; }
  .progress progress { width: 100%; height: .5rem; accent-color: var(--primary); }
  .summary { font-size: .9rem; }
  details { font-size: .8rem; color: var(--muted-foreground); }
  details ul { margin: .4rem 0 0; padding-left: 1.2rem; }
  .error-msg { color: #ef4444; font-size: .85rem; white-space: pre-wrap; }
  .dialog-footer { display: flex; align-items: center; justify-content: space-between; padding: .75rem 1rem; background: var(--sidebar-bg); border-top: 1px solid var(--border); border-radius: 0 0 var(--radius-lg, 12px) var(--radius-lg, 12px); }
  .actions { display: flex; gap: .5rem; }
  .shortcuts { display: flex; gap: 1.5rem; font-size: .7rem; color: var(--muted-foreground); }
  kbd { background: var(--bg); border: 1px solid var(--border); border-radius: 3px; padding: 0 4px; font-family: inherit; font-size: .8em; }
  .primary-btn { background: var(--primary); color: var(--primary-foreground); border: none; border-radius: var(--radius); padding: .4rem .9rem; font-size: .85rem; font-weight: 500; cursor: pointer; }
  .primary-btn:hover:not(:disabled) { background: var(--primary-hover); }
  .secondary-btn { background: var(--bg); color: inherit; border: 1px solid var(--border); border-radius: var(--radius); padding: .4rem .9rem; font-size: .85rem; cursor: pointer; }
  .primary-btn:disabled, .secondary-btn:disabled { opacity: .5; cursor: not-allowed; }
</style>
