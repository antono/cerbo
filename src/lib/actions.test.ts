import { describe, expect, it, vi } from 'vitest';
import {
  ACTIONS,
  HELP_ACTIONS,
  HELP_NAVIGATION,
  OVERFLOW_MENU,
  PAGE_MENU,
  actionById,
  actionForKey,
  actionIcon,
  actionLabel,
  availableActions,
  dispatchHotkey,
  hotkeySignatures,
  hotkeyText,
  matchesHotkey,
  pressedKey,
  type ActionContext,
  type KeyContext,
} from './actions';

// Under vitest there is no browser, so the primary modifier is Ctrl.
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

function context(state: Partial<ActionContext> = {}): ActionContext {
  const ops = {
    toggleDialog: vi.fn(),
    openSearch: vi.fn(),
    toggleSearch: vi.fn(),
    addVault: vi.fn(),
    toggleLeftSidebar: vi.fn(),
    toggleRightSidebar: vi.fn(),
    toggleEditorMode: vi.fn(),
    setTheme: vi.fn(),
    goBack: vi.fn(),
    goForward: vi.fn(),
    nextPage: vi.fn(),
    prevPage: vi.fn(),
    renamePage: vi.fn(),
    movePage: vi.fn(),
    deletePage: vi.fn(),
    copyPageLink: vi.fn(),
    revealPage: vi.fn(),
    newFolder: vi.fn(),
    quit: vi.fn(),
  };
  return {
    hasVault: true,
    currentUuid: 'page-137',
    theme: 'light',
    editorTab: 'preview',
    showLeftSidebar: true,
    showRightSidebar: true,
    canGoBack: true,
    canGoForward: true,
    ...ops,
    ...state,
  };
}

const editing: KeyContext = { dialogOpen: false, inputFocused: true, editorTab: 'write' };
const previewing: KeyContext = { dialogOpen: false, inputFocused: false, editorTab: 'preview' };

describe('action registry', () => {
  it('has unique ids and unique hotkeys', () => {
    const ids = ACTIONS.map((a) => a.id);
    expect(new Set(ids).size).toBe(ids.length);
    const signatures = ACTIONS.flatMap((a) => (a.hotkey ? hotkeySignatures(a.hotkey) : []));
    expect(signatures.length).toBeGreaterThan(0);
    expect(new Set(signatures).size).toBe(signatures.length);
  });

  it('lists every action the toolbar, its menus and command mode need', () => {
    for (const id of [
      ...PAGE_MENU,
      ...OVERFLOW_MENU,
      ...HELP_ACTIONS,
      ...HELP_NAVIGATION,
      'vault.switch',
      'vault.add',
      'panel.left',
      'panel.right',
      'nav.back',
      'nav.forward',
      'nav.next',
      'nav.prev',
      'search.pages',
      'search.commands',
      'page.new',
      'page.mode',
    ]) {
      expect(() => actionById(id)).not.toThrow();
    }
    const ctx = context();
    for (const action of ACTIONS) {
      expect(actionLabel(action, ctx)).not.toBe('');
      expect(actionIcon(action, ctx)).toBeTruthy();
    }
  });

  it('keeps Delete last in the page menu, marked destructive', () => {
    expect(PAGE_MENU.at(-1)).toBe('page.delete');
    expect(actionById('page.delete').destructive).toBe(true);
    expect(PAGE_MENU.slice(0, -1).some((id) => actionById(id).destructive)).toBe(false);
  });

  it('hides every action with `when` when no page is open', () => {
    const empty = context({ hasVault: false, currentUuid: null, canGoBack: false, canGoForward: false });
    const conditional = ACTIONS.filter((a) => a.when);
    expect(conditional.length).toBeGreaterThan(0);
    for (const action of conditional) expect(action.when!(empty), action.id).toBe(false);
    expect(availableActions(empty).every((a) => !a.when)).toBe(true);
  });

  it('hides page actions in an open vault with no page, and keeps vault actions', () => {
    const ids = availableActions(context({ currentUuid: null })).map((a) => a.id);
    for (const id of ['page.rename', 'page.move', 'page.delete', 'page.copyLink', 'page.reveal', 'page.mode']) {
      expect(ids).not.toContain(id);
    }
    expect(ids).toContain('page.new');
    expect(ids).toContain('folder.new');
    expect(ids).toContain('sparql.console');
  });

  it('names the theme entry after the theme it switches to', () => {
    const theme = actionById('theme.toggle');
    expect(actionLabel(theme, context({ theme: 'dark' }))).toBe('Light theme');
    expect(actionLabel(theme, context({ theme: 'light' }))).toBe('Dark theme');
    expect(actionIcon(theme, context({ theme: 'dark' }))).not.toBe(actionIcon(theme, context({ theme: 'light' })));
    expect(theme.synonyms).toEqual(expect.arrayContaining(['theme', 'dark', 'light']));
    expect(hotkeyText(theme.hotkey!, false)).toBe('Ctrl+T');
  });

  it('shows the SPARQL console with Ctrl+Shift+E', () => {
    expect(hotkeyText(actionById('sparql.console').hotkey!, false)).toBe('Ctrl+Shift+E');
    expect(hotkeyText(actionById('sparql.console').hotkey!, true)).toBe('⌘+Shift+E');
    // Plain preview keys stay lower case; upper case means Shift.
    expect(hotkeyText(actionById('page.rename').hotkey!, false)).toBe('r');
    expect(hotkeyText(actionById('page.move').hotkey!, false)).toBe('m');
    expect(hotkeyText(actionById('nav.next').hotkey!, false)).toBe('J');
  });
});

describe('hotkeys', () => {
  it('matches Ctrl+\\ on e.code, whatever the layout reports as e.key', () => {
    const tree = actionById('panel.left').hotkey!;
    // German layout: the key next to Enter is `#`, but the code is Backslash.
    expect(matchesHotkey(key({ code: 'Backslash', key: '#', ctrlKey: true }), tree, false)).toBe(true);
    expect(matchesHotkey(key({ code: 'Backslash', key: '\\', ctrlKey: true }), tree, false)).toBe(true);
    // A `\` typed through another physical key is not the shortcut.
    expect(matchesHotkey(key({ code: 'IntlBackslash', key: '\\', ctrlKey: true }), tree, false)).toBe(false);
    expect(matchesHotkey(key({ code: 'Backslash', key: '\\' }), tree, false)).toBe(false);
  });

  it('tells Ctrl+\\ from Ctrl+Shift+\\', () => {
    const shifted = key({ code: 'Backslash', key: '|', ctrlKey: true, shiftKey: true });
    expect(actionForKey(shifted, previewing, ACTIONS, false)?.id).toBe('panel.right');
    const plain = key({ code: 'Backslash', key: '\\', ctrlKey: true });
    expect(actionForKey(plain, previewing, ACTIONS, false)?.id).toBe('panel.left');
  });

  it('uses Cmd instead of Ctrl on Mac', () => {
    const tree = actionById('panel.left').hotkey!;
    expect(matchesHotkey(key({ code: 'Backslash', metaKey: true }), tree, true)).toBe(true);
    expect(matchesHotkey(key({ code: 'Backslash', ctrlKey: true }), tree, true)).toBe(false);
    expect(hotkeyText(tree, true)).toBe('⌘+\\');
  });

  it('tells Ctrl+P, Ctrl+Shift+P, Ctrl+E and Ctrl+Shift+E apart', () => {
    const find = (init: Partial<KeyboardEvent>) => actionForKey(key(init), previewing, ACTIONS, false)?.id;
    expect(find({ key: 'p', ctrlKey: true })).toBe('search.pages');
    expect(find({ key: 'P', ctrlKey: true, shiftKey: true })).toBe('search.commands');
    expect(find({ key: 'e', ctrlKey: true })).toBe('page.mode');
    expect(find({ key: 'E', ctrlKey: true, shiftKey: true })).toBe('sparql.console');
    expect(find({ key: 'o', ctrlKey: true })).toBe('vault.switch');
    expect(find({ key: 'O', ctrlKey: true, shiftKey: true })).toBe('vault.add');
  });

  it('runs the panel and mode keys while the editor has focus', () => {
    const ctx = context({ editorTab: 'write' });
    const press = (init: Partial<KeyboardEvent>) => {
      const e = key(init);
      const action = dispatchHotkey(e, editing, ctx, ACTIONS, false);
      expect(e.preventDefault).toHaveBeenCalled();
      return action?.id;
    };
    expect(press({ code: 'Backslash', key: '\\', ctrlKey: true })).toBe('panel.left');
    expect(ctx.toggleLeftSidebar).toHaveBeenCalledTimes(1);
    expect(press({ code: 'Backslash', key: '|', ctrlKey: true, shiftKey: true })).toBe('panel.right');
    expect(ctx.toggleRightSidebar).toHaveBeenCalledTimes(1);
    expect(press({ key: 'e', ctrlKey: true })).toBe('page.mode');
    expect(ctx.toggleEditorMode).toHaveBeenCalledTimes(1);
  });

  it('keeps preview-only keys out of the editor', () => {
    const ctx = context({ editorTab: 'write' });
    for (const init of [{ key: 'r' }, { key: 'm' }, { key: 'Delete' }, { key: 'J', shiftKey: true }]) {
      const e = key(init);
      expect(dispatchHotkey(e, editing, ctx, ACTIONS, false)).toBeNull();
      expect(e.preventDefault).not.toHaveBeenCalled();
    }
    expect(ctx.renamePage).not.toHaveBeenCalled();
    expect(ctx.deletePage).not.toHaveBeenCalled();
  });

  it('runs preview keys in preview', () => {
    const ctx = context();
    const find = (init: Partial<KeyboardEvent>) => dispatchHotkey(key(init), previewing, ctx, ACTIONS, false)?.id;
    expect(find({ key: 'r' })).toBe('page.rename');
    expect(find({ key: 'm' })).toBe('page.move');
    expect(find({ key: 'Backspace' })).toBe('page.delete');
    expect(find({ key: 'J', shiftKey: true })).toBe('nav.next');
    expect(find({ key: 'K', shiftKey: true })).toBe('nav.prev');
    // Plain j/k scroll the preview; the layout handles them, not the registry.
    expect(find({ key: 'j' })).toBeUndefined();
    // Ctrl+R is not rename.
    expect(find({ key: 'r', ctrlKey: true })).toBeUndefined();
  });

  it('lets only dialog toggles through while a dialog is open', () => {
    const open: KeyContext = { ...previewing, dialogOpen: true };
    const find = (init: Partial<KeyboardEvent>) => actionForKey(key(init), open, ACTIONS, false)?.id;
    expect(find({ key: 'F1' })).toBe('help.shortcuts');
    expect(find({ key: 'n', ctrlKey: true })).toBe('page.new');
    expect(find({ key: 't', ctrlKey: true })).toBe('theme.toggle');
    expect(find({ key: 'E', ctrlKey: true, shiftKey: true })).toBe('sparql.console');
    expect(find({ key: 'P', ctrlKey: true, shiftKey: true })).toBe('search.commands');
    expect(find({ key: 'p', ctrlKey: true })).toBeUndefined();
    expect(find({ code: 'Backslash', ctrlKey: true })).toBeUndefined();
    expect(find({ key: 'q', ctrlKey: true })).toBeUndefined();
    expect(find({ key: 'r' })).toBeUndefined();
  });

  it('eats Ctrl+E with no page open without running it', () => {
    const ctx = context({ currentUuid: null });
    const e = key({ key: 'e', ctrlKey: true });
    expect(dispatchHotkey(e, previewing, ctx, ACTIONS, false)?.id).toBe('page.mode');
    expect(e.preventDefault).toHaveBeenCalled();
    expect(ctx.toggleEditorMode).not.toHaveBeenCalled();
  });

  it('opens command mode with `>` and plain search without it', () => {
    const ctx = context();
    dispatchHotkey(key({ key: 'P', ctrlKey: true, shiftKey: true }), previewing, ctx, ACTIONS, false);
    expect(ctx.toggleSearch).toHaveBeenLastCalledWith('>');
    dispatchHotkey(key({ key: 'p', ctrlKey: true }), previewing, ctx, ACTIONS, false);
    expect(ctx.toggleSearch).toHaveBeenLastCalledWith('');
  });

  it('matches letter hotkeys on a Russian layout by the physical key', () => {
    const find = (init: Partial<KeyboardEvent>) => actionForKey(key(init), previewing, ACTIONS, false)?.id;
    // ЙЦУКЕН: P is `з`, N is `т`, E is `у`, T is `е`, O is `щ`, Q is `й`.
    expect(find({ key: 'з', code: 'KeyP', ctrlKey: true })).toBe('search.pages');
    expect(find({ key: 'т', code: 'KeyN', ctrlKey: true })).toBe('page.new');
    expect(find({ key: 'у', code: 'KeyE', ctrlKey: true })).toBe('page.mode');
    expect(find({ key: 'е', code: 'KeyT', ctrlKey: true })).toBe('theme.toggle');
    expect(find({ key: 'щ', code: 'KeyO', ctrlKey: true })).toBe('vault.switch');
    expect(find({ key: 'й', code: 'KeyQ', ctrlKey: true })).toBe('app.quit');
    expect(find({ key: 'З', code: 'KeyP', ctrlKey: true, shiftKey: true })).toBe('search.commands');
    expect(find({ key: 'У', code: 'KeyE', ctrlKey: true, shiftKey: true })).toBe('sparql.console');
    expect(find({ key: 'Щ', code: 'KeyO', ctrlKey: true, shiftKey: true })).toBe('vault.add');
    // Preview keys: R is `к`, M is `ь`, J is `о`, K is `л`.
    expect(find({ key: 'к', code: 'KeyR' })).toBe('page.rename');
    expect(find({ key: 'ь', code: 'KeyM' })).toBe('page.move');
    expect(find({ key: 'О', code: 'KeyJ', shiftKey: true })).toBe('nav.next');
    expect(find({ key: 'Л', code: 'KeyK', shiftKey: true })).toBe('nav.prev');
  });

  it('keeps the Latin letter of the layout over the physical key', () => {
    const find = (init: Partial<KeyboardEvent>) => actionForKey(key(init), previewing, ACTIONS, false)?.id;
    // Dvorak: the key in the QWERTY `R` place types `p`.
    expect(find({ key: 'p', code: 'KeyR', ctrlKey: true })).toBe('search.pages');
    // A non-letter does not fall back to the code (Dvorak `,` sits on KeyW).
    expect(find({ key: ',', code: 'KeyE', ctrlKey: true })).toBeUndefined();
    expect(pressedKey(key({ key: 'Delete', code: 'Delete' }))).toBe('delete');
    expect(pressedKey(key({ key: 'з', code: 'KeyP' }))).toBe('p');
    expect(pressedKey(key({ key: 'з', code: '' }))).toBe('з');
  });

  it('switches to the other theme', () => {
    const ctx = context({ theme: 'dark' });
    dispatchHotkey(key({ key: 't', ctrlKey: true }), previewing, ctx, ACTIONS, false);
    expect(ctx.setTheme).toHaveBeenCalledWith('light');
  });
});

describe('shortcuts help', () => {
  it('lists the panel, mode and command mode hotkeys (spec `hotkeys-help`)', () => {
    const listed = [...HELP_ACTIONS, ...HELP_NAVIGATION].map((id) => hotkeyText(actionById(id).hotkey!, false));
    for (const k of ['Ctrl+O', 'Ctrl+\\', 'Ctrl+Shift+\\', 'Ctrl+E', 'Ctrl+Shift+P', 'Ctrl+P', 'F1', 'Alt+←', 'J']) {
      expect(listed).toContain(k);
    }
  });

  it('lists a hotkey for every action it names', () => {
    for (const id of [...HELP_ACTIONS, ...HELP_NAVIGATION]) expect(actionById(id).hotkey, id).toBeDefined();
  });
});
