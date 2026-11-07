import { beforeEach, describe, expect, it, vi } from 'vitest';

// Spec `editor-position-restore`: a vault switch or quit waits for the page
// editor to store where editing ended before any state changes.
const calls: string[] = [];
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (cmd: string) => {
    calls.push(`invoke ${cmd}`);
    if (cmd === 'vault_open') throw new Error('stop here');
    return undefined;
  }),
}));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));
vi.mock('@tauri-apps/api/path', () => ({ homeDir: vi.fn() }));
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => ({ destroy: vi.fn() }) }));

const { onEditorLeave, openVault, quitApp, runEditorLeaveHooks, app } = await import('./stores.svelte');

/** A hook that records when it finished, after a macrotask. */
function slowHook(name: string) {
  return () => new Promise<void>((resolve) => setTimeout(() => {
    calls.push(name);
    resolve();
  }, 0));
}

describe('editor leave hooks', () => {
  beforeEach(() => {
    calls.length = 0;
  });

  it('runs every hook, and a failing one blocks nothing', async () => {
    const stopA = onEditorLeave(slowHook('a'));
    const stopB = onEditorLeave(() => Promise.reject(new Error('save failed')));
    const stopC = onEditorLeave(slowHook('c'));
    await runEditorLeaveHooks();
    expect(calls.sort()).toEqual(['a', 'c']);
    stopA();
    stopB();
    stopC();
  });

  it('stops running a hook once unregistered', async () => {
    const stop = onEditorLeave(slowHook('gone'));
    stop();
    await runEditorLeaveHooks();
    expect(calls).toEqual([]);
  });

  it('openVault waits for the hooks before opening the vault', async () => {
    const stop = onEditorLeave(() => {
      // Still the old state while the hook runs.
      calls.push(`hook loading=${app.loading}`);
      return slowHook('saved')();
    });
    await openVault('vault-137');
    expect(calls.slice(0, 3)).toEqual(['hook loading=false', 'saved', 'invoke vault_open']);
    stop();
  });

  it('quitApp waits for the hooks before exiting', async () => {
    const stop = onEditorLeave(slowHook('saved'));
    await quitApp();
    expect(calls[0]).toBe('saved');
    expect(calls).toContain('invoke app_exit');
    expect(calls.indexOf('saved')).toBeLessThan(calls.indexOf('invoke app_exit'));
    stop();
  });
});
