import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  changeWidthClass,
  dockedWidths,
  isDocked,
  maxPanelWidth,
  panelShown,
  showPanel,
  togglePanel,
  widthClassFor,
  type PanelState,
} from './layout';
import { overflowMenuIds } from './toolbar';

// Spec `desktop-responsive-layout` (change `unify-window-toolbar`, tasks 4.1–4.4).

describe('widthClassFor', () => {
  it('splits at 1000px and 700px', () => {
    expect(widthClassFor(1200)).toBe('wide');
    expect(widthClassFor(1000)).toBe('wide');
    expect(widthClassFor(999)).toBe('medium');
    expect(widthClassFor(800)).toBe('medium');
    expect(widthClassFor(700)).toBe('medium');
    expect(widthClassFor(699)).toBe('narrow');
    expect(widthClassFor(390)).toBe('narrow');
    expect(widthClassFor(360)).toBe('narrow');
  });
});

const wide: PanelState = { widthClass: 'wide', showLeft: true, showRight: true, overlay: null };

describe('docking per width class', () => {
  it('docks both panels in wide, only the tree in medium, nothing in narrow', () => {
    expect(isDocked('tree', 'wide')).toBe(true);
    expect(isDocked('right', 'wide')).toBe(true);
    expect(isDocked('tree', 'medium')).toBe(true);
    expect(isDocked('right', 'medium')).toBe(false);
    expect(isDocked('tree', 'narrow')).toBe(false);
    expect(isDocked('right', 'narrow')).toBe(false);
  });

  it('a narrow window shows neither panel until one is opened', () => {
    const s = changeWidthClass('narrow', wide);
    expect(panelShown('tree', s)).toBe(false);
    expect(panelShown('right', s)).toBe(false);
  });
});

describe('togglePanel', () => {
  it('wide: flips the docked flag and asks to persist it', () => {
    const { state, persist } = togglePanel('right', wide);
    expect(state.showRight).toBe(false);
    expect(state.overlay).toBeNull();
    expect(persist).toBe(true);
  });

  it('medium: the tree stays docked, the right panel floats without touching the stored flag', () => {
    const medium = { ...wide, widthClass: 'medium' as const, showRight: false };
    const tree = togglePanel('tree', medium);
    expect(tree.state.showLeft).toBe(false);
    expect(tree.persist).toBe(true);

    const right = togglePanel('right', medium);
    expect(right.state.overlay).toBe('right');
    expect(right.state.showRight).toBe(false);
    expect(right.persist).toBe(false);
    expect(panelShown('right', right.state)).toBe(true);
  });

  it('narrow: the toggle opens and closes its overlay', () => {
    const narrow = changeWidthClass('narrow', wide);
    const open = togglePanel('tree', narrow);
    expect(open.state.overlay).toBe('tree');
    expect(open.persist).toBe(false);
    const closed = togglePanel('tree', open.state);
    expect(closed.state.overlay).toBeNull();
    expect(closed.state.showLeft).toBe(true);
  });

  it('opening one overlay closes the other', () => {
    const narrow = { ...changeWidthClass('narrow', wide), overlay: 'tree' as const };
    const { state } = togglePanel('right', narrow);
    expect(state.overlay).toBe('right');
    expect(panelShown('tree', state)).toBe(false);
  });

  it('overlays never change the stored docked state', () => {
    let s = changeWidthClass('narrow', { ...wide, showLeft: false, showRight: true });
    for (const panel of ['tree', 'right', 'tree', 'tree', 'right'] as const) {
      const next = togglePanel(panel, s);
      expect(next.persist).toBe(false);
      s = next.state;
    }
    expect(s.showLeft).toBe(false);
    expect(s.showRight).toBe(true);
  });
});

describe('showPanel', () => {
  it('opens a hidden panel and leaves a shown one alone', () => {
    const narrow = changeWidthClass('narrow', wide);
    expect(showPanel('tree', narrow).state.overlay).toBe('tree');
    const open = { ...narrow, overlay: 'tree' as const };
    expect(showPanel('tree', open)).toEqual({ state: open, persist: false });
    expect(showPanel('tree', { ...wide, showLeft: false }).state.showLeft).toBe(true);
  });
});

describe('changeWidthClass', () => {
  it('closes overlays on crossing a boundary', () => {
    const s = { ...wide, widthClass: 'medium' as const, overlay: 'right' as const };
    expect(changeWidthClass('narrow', s).overlay).toBeNull();
    expect(changeWidthClass('medium', s)).toBe(s);
  });

  it('back to wide shows the remembered docked panels again', () => {
    let s = changeWidthClass('medium', wide);
    s = togglePanel('right', s).state; // float the panel in medium
    s = changeWidthClass('narrow', s);
    s = changeWidthClass('wide', s);
    expect(panelShown('right', s)).toBe(true);
    expect(s.overlay).toBeNull();
  });
});

describe('maxPanelWidth', () => {
  it('leaves the editor 360px', () => {
    expect(maxPanelWidth(1000, 280, 200, 600)).toBe(360);
    expect(maxPanelWidth(1600, 280, 200, 600)).toBe(600);
    expect(maxPanelWidth(700, 280, 200, 600)).toBe(200);
  });
});

describe('dockedWidths', () => {
  it('keeps the stored widths when the editor has room', () => {
    expect(dockedWidths(1920, 600, 600)).toEqual({ tree: 600, right: 600 });
    expect(dockedWidths(1280, 280, 300)).toEqual({ tree: 280, right: 300 });
  });

  it('shrinks the right panel first, then the tree, so the editor keeps 360px', () => {
    // 1100 - 360 = 740 for both panels.
    expect(dockedWidths(1100, 600, 600)).toEqual({ tree: 490, right: 250 });
    expect(dockedWidths(1100, 400, 600)).toEqual({ tree: 400, right: 340 });
    // 1000 - 360 = 640.
    expect(dockedWidths(1000, 600, 600)).toEqual({ tree: 390, right: 250 });
  });

  it('never goes below the panel minimums', () => {
    expect(dockedWidths(500, 600, 600)).toEqual({ tree: 200, right: 250 });
  });

  it('handles a single docked panel', () => {
    expect(dockedWidths(700, 600, 0)).toEqual({ tree: 340, right: 0 });
    expect(dockedWidths(1000, 0, 600)).toEqual({ tree: 0, right: 600 });
    expect(dockedWidths(800, 0, 600)).toEqual({ tree: 0, right: 440 });
  });

  it('leaves the widths alone before the window is measured', () => {
    expect(dockedWidths(0, 600, 600)).toEqual({ tree: 600, right: 600 });
  });
});

describe('overflowMenuIds', () => {
  it('adds forward and the right panel toggle in the narrow layout only', () => {
    const base = ['theme.toggle', 'app.quit'];
    expect(overflowMenuIds(base, 'wide')).toEqual(base);
    expect(overflowMenuIds(base, 'medium')).toEqual(base);
    expect(overflowMenuIds(base, 'narrow')).toEqual(['nav.forward', 'panel.right', ...base]);
  });
});

// The same rules through the app store, as the toolbar and hotkeys use them.
const invoke = vi.fn(async (_cmd: string, _args?: unknown) => undefined);
vi.mock('@tauri-apps/api/core', () => ({ invoke: (cmd: string, args?: unknown) => invoke(cmd, args) }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));
vi.mock('@tauri-apps/api/path', () => ({ homeDir: vi.fn() }));
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => ({ destroy: vi.fn() }) }));
vi.mock('mode-watcher', () => ({ mode: { current: 'light' }, setMode: vi.fn() }));

const store = await import('./stores.svelte');
const { appActions } = await import('./action-host');
const { app } = store;

describe('layout state in the app store', () => {
  beforeEach(() => {
    invoke.mockClear();
    app.widthClass = 'wide';
    app.overlay = null;
    app.showLeftSidebar = true;
    app.showRightSidebar = true;
    app.currentUuid = 'page-137';
  });

  it('setWindowWidth follows the window across boundaries and closes overlays', () => {
    store.setWindowWidth(800);
    expect(app.widthClass).toBe('medium');
    store.toggleRightSidebar();
    expect(app.overlay).toBe('right');
    store.setWindowWidth(390);
    expect(app.widthClass).toBe('narrow');
    expect(app.overlay).toBeNull();
    store.setWindowWidth(1200);
    expect(app.widthClass).toBe('wide');
    expect(app.showRightSidebar).toBe(true);
  });

  it('narrow toggles open overlays and never save settings', () => {
    store.setWindowWidth(390);
    store.toggleLeftSidebar();
    expect(app.overlay).toBe('tree');
    expect(appActions.showLeftSidebar).toBe(true);
    store.toggleRightSidebar();
    expect(app.overlay).toBe('right');
    expect(appActions.showLeftSidebar).toBe(false);
    store.closeOverlay();
    expect(app.overlay).toBeNull();
    expect(app.showLeftSidebar).toBe(true);
    expect(app.showRightSidebar).toBe(true);
    expect(invoke).not.toHaveBeenCalledWith('ui_settings_save', expect.anything());
  });

  it('wide toggles still persist the docked state', () => {
    store.toggleRightSidebar();
    expect(app.showRightSidebar).toBe(false);
    expect(invoke).toHaveBeenLastCalledWith('ui_settings_save', expect.objectContaining({ rightSidebarVisible: false }));
  });

  it('Reveal in tree opens the drawer in the narrow layout', () => {
    store.setWindowWidth(390);
    store.revealFolder('Projects');
    expect(app.overlay).toBe('tree');
    expect(app.revealedFolder).toBe('Projects');
  });
});
