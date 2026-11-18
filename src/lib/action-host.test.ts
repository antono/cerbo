import { beforeEach, describe, expect, it, vi } from 'vitest';

// The registry bound to the real store: the new hotkeys change app state and
// persist the panel toggles (spec `keyboard-shortcuts`, `desktop-window-toolbar`).
const invoke = vi.fn(async (_cmd: string, _args?: unknown) => undefined);
vi.mock('@tauri-apps/api/core', () => ({ invoke: (cmd: string, args?: unknown) => invoke(cmd, args) }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));
vi.mock('@tauri-apps/api/path', () => ({ homeDir: vi.fn() }));
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => ({ destroy: vi.fn() }) }));
const setMode = vi.fn();
vi.mock('mode-watcher', () => ({ mode: { current: 'light' }, setMode: (m: string) => setMode(m) }));

const { app, onEditorModeToggle } = await import('./stores.svelte');
const { appActions } = await import('./action-host');
const { dispatchHotkey } = await import('./actions');
const { anyDialogOpen } = await import('./hotkeys');

function key(init: Partial<KeyboardEvent>): KeyboardEvent {
  return {
    key: '',
    code: '',
    ctrlKey: false,
    metaKey: false,
    shiftKey: false,
    altKey: false,
    preventDefault: vi.fn(),
    ...init,
  } as unknown as KeyboardEvent;
}

const inEditor = { dialogOpen: false, inputFocused: true, editorTab: 'write' as const };

describe('registry bound to the app store', () => {
  beforeEach(() => {
    invoke.mockClear();
    app.showLeftSidebar = true;
    app.showRightSidebar = true;
    app.currentUuid = 'page-137';
    app.editorTab = 'write';
    app.showSearch = false;
    app.searchPrefill = '';
  });

  it('"Search pages" run from command mode reopens search empty', async () => {
    const { openSearch, closeAllDialogs } = await import('./stores.svelte');
    const { runCommand } = await import('./command-mode');
    const { ACTIONS } = await import('./actions');
    openSearch('>');
    const opens = app.searchOpens;
    runCommand(ACTIONS.find((a) => a.id === 'search.pages')!, appActions, closeAllDialogs);
    // Search stays mounted, so the open counter is what makes it take the
    // new, empty prefill.
    expect(app.showSearch).toBe(true);
    expect(app.searchPrefill).toBe('');
    expect(app.searchOpens).toBe(opens + 1);
  });

  it('Ctrl+\\ hides and shows the page tree from the editor, and saves it', () => {
    dispatchHotkey(key({ code: 'Backslash', key: '\\', ctrlKey: true }), inEditor, appActions, undefined, false);
    expect(app.showLeftSidebar).toBe(false);
    expect(invoke).toHaveBeenLastCalledWith('ui_settings_save', expect.objectContaining({ leftSidebarVisible: false }));
    dispatchHotkey(key({ code: 'Backslash', key: '\\', ctrlKey: true }), inEditor, appActions, undefined, false);
    expect(app.showLeftSidebar).toBe(true);
    expect(app.showRightSidebar).toBe(true);
  });

  it('Ctrl+Shift+\\ hides the right panel and saves it', () => {
    dispatchHotkey(key({ code: 'Backslash', key: '|', ctrlKey: true, shiftKey: true }), inEditor, appActions, undefined, false);
    expect(app.showRightSidebar).toBe(false);
    expect(app.showLeftSidebar).toBe(true);
    expect(invoke).toHaveBeenLastCalledWith('ui_settings_save', expect.objectContaining({ rightSidebarVisible: false }));
  });

  it('Ctrl+E goes through the editor so it can store the caret', () => {
    const toggle = vi.fn();
    const stop = onEditorModeToggle(toggle);
    dispatchHotkey(key({ key: 'e', ctrlKey: true }), inEditor, appActions, undefined, false);
    expect(toggle).toHaveBeenCalledTimes(1);
    stop();
  });

  it('Ctrl+E without an editor flips the mode both ways', () => {
    app.editorTab = 'preview';
    const preview = { dialogOpen: false, inputFocused: false, editorTab: 'preview' as const };
    dispatchHotkey(key({ key: 'e', ctrlKey: true }), preview, appActions, undefined, false);
    expect(app.editorTab).toBe('write');
    dispatchHotkey(key({ key: 'e', ctrlKey: true }), inEditor, appActions, undefined, false);
    expect(app.editorTab).toBe('preview');
  });

  it('Ctrl+Shift+P opens search with `>` and closes it when open', () => {
    const preview = { dialogOpen: false, inputFocused: false, editorTab: 'preview' as const };
    dispatchHotkey(key({ key: 'P', ctrlKey: true, shiftKey: true }), preview, appActions, undefined, false);
    expect(app.showSearch).toBe(true);
    expect(app.searchPrefill).toBe('>');
    dispatchHotkey(key({ key: 'P', ctrlKey: true, shiftKey: true }), { ...preview, dialogOpen: true }, appActions, undefined, false);
    expect(app.showSearch).toBe(false);
  });

  it('the theme action switches mode-watcher and saves', () => {
    appActions.setTheme('dark');
    expect(setMode).toHaveBeenCalledWith('dark');
    expect(app.theme).toBe('dark');
  });

  describe('with a toolbar menu open (one transient element)', () => {
    const where = () => ({ dialogOpen: anyDialogOpen(app), inputFocused: false, editorTab: 'preview' as const });

    beforeEach(() => {
      app.editorTab = 'preview';
      app.showHelp = false;
      app.renameUuid = null;
      app.moveUuid = null;
      app.confirmDeleteUuid = null;
      app.toolbarMenu = 'more';
    });

    it('counts as an open transient element', () => {
      expect(anyDialogOpen(app)).toBe(true);
      app.toolbarMenu = null;
      expect(anyDialogOpen(app)).toBe(false);
    });

    it('plain-key page hotkeys do not run behind the menu', () => {
      app.toolbarMenu = 'page';
      expect(dispatchHotkey(key({ key: 'r', code: 'KeyR' }), where(), appActions, undefined, false)).toBeNull();
      expect(dispatchHotkey(key({ key: 'm', code: 'KeyM' }), where(), appActions, undefined, false)).toBeNull();
      expect(dispatchHotkey(key({ key: 'Delete', code: 'Delete' }), where(), appActions, undefined, false)).toBeNull();
      expect(app.renameUuid).toBeNull();
      expect(app.moveUuid).toBeNull();
      expect(app.confirmDeleteUuid).toBeNull();
      expect(app.toolbarMenu).toBe('page');
    });

    it('F1 closes the menu and opens help', () => {
      dispatchHotkey(key({ key: 'F1', code: 'F1' }), where(), appActions, undefined, false);
      expect(app.showHelp).toBe(true);
      expect(app.toolbarMenu).toBeNull();
    });

    it('Ctrl+Shift+P closes the menu before command mode opens', () => {
      app.showSearch = false;
      dispatchHotkey(key({ key: 'P', code: 'KeyP', ctrlKey: true, shiftKey: true }), where(), appActions, undefined, false);
      expect(app.showSearch).toBe(true);
      expect(app.toolbarMenu).toBeNull();
    });

    it('closeAllDialogs closes the menu', async () => {
      const { closeAllDialogs } = await import('./stores.svelte');
      closeAllDialogs();
      expect(app.toolbarMenu).toBeNull();
    });
  });
});
