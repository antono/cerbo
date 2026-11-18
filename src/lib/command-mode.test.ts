import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ACTIONS, actionLabel, hotkeyText, type ActionContext } from './actions';
import {
  clearRecentActions,
  commandResults,
  commandTerm,
  isCommandQuery,
  matchRank,
  recentActionIds,
  runCommand,
} from './command-mode';

function context(state: Partial<ActionContext> = {}): ActionContext {
  return {
    hasVault: true,
    currentUuid: 'page-137',
    theme: 'light',
    editorTab: 'preview',
    showLeftSidebar: true,
    showRightSidebar: true,
    rightPanelTab: 'links',
    canGoBack: true,
    canGoForward: true,
    toggleDialog: vi.fn(),
    openSearch: vi.fn(),
    toggleSearch: vi.fn(),
    addVault: vi.fn(),
    toggleLeftSidebar: vi.fn(),
    toggleRightSidebar: vi.fn(),
    useRightPanelTab: vi.fn(),
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
    ...state,
  };
}

const labels = (query: string, ctx = context()) => commandResults(query, ctx, ACTIONS, []).map((a) => actionLabel(a, ctx));
const ids = (query: string, ctx = context(), recent: string[] = []) => commandResults(query, ctx, ACTIONS, recent).map((a) => a.id);

beforeEach(() => clearRecentActions());

describe('command query', () => {
  it('starts with >', () => {
    expect(isCommandQuery('>')).toBe(true);
    expect(isCommandQuery('>folder')).toBe(true);
    expect(isCommandQuery('folder')).toBe(false);
    // Spec "Back to pages": deleting the `>` returns to page search.
    expect(isCommandQuery('the')).toBe(false);
    expect(isCommandQuery('')).toBe(false);
  });

  it('takes the text after >, trimmed and lower-cased', () => {
    expect(commandTerm('>  SPARQL ')).toBe('sparql');
    expect(commandTerm('>')).toBe('');
  });
});

describe('matching', () => {
  it('lists every action that applies for a bare >, except the command mode hotkey itself', () => {
    const listed = ids('>');
    expect(listed).not.toContain('search.commands');
    expect(listed).toEqual(ACTIONS.filter((a) => !a.hideInCommands).map((a) => a.id));
  });

  it('matches a substring of the label, ignoring case', () => {
    expect(labels('>SPARQL')).toEqual(['SPARQL console']);
    expect(labels('>olde')).toContain('New folder');
  });

  it('matches a substring of a synonym', () => {
    expect(ids('>clipboard')).toEqual(['page.copyLink']);
    expect(ids('>directory')).toEqual(['folder.new']);
  });

  it('finds nothing for text in neither label nor synonyms', () => {
    expect(ids('>zzz137')).toEqual([]);
  });

  it('lists label matches before synonym-only matches', () => {
    const ctx = context();
    const action = ACTIONS.find((a) => a.id === 'page.move')!;
    expect(matchRank(action, ctx, 'move')).toBe(2);
    expect(matchRank(action, ctx, 'folder')).toBe(1);
    expect(matchRank(action, ctx, 'zzz')).toBe(0);
    // `>folder`: "New folder" (label) comes before "Move page" (synonym "folder").
    expect(ids('>folder')[0]).toBe('folder.new');
    expect(ids('>folder')).toContain('page.move');
  });

  it('shows the hotkey of a listed action (spec "Hotkey shown")', () => {
    const [sparql] = commandResults('>sparql', context(), ACTIONS, []);
    expect(sparql.hotkey && hotkeyText(sparql.hotkey, false)).toBe('Ctrl+Shift+E');
  });
});

describe('the "Dark theme" synonym case', () => {
  it('puts "Dark theme" first for >dark while the light theme is active', () => {
    expect(labels('>dark', context({ theme: 'light' }))[0]).toBe('Dark theme');
  });

  it('finds the theme switch by "theme", "dark" and "light"', () => {
    for (const q of ['>theme', '>dark', '>light']) expect(ids(q)).toContain('theme.toggle');
  });

  it('names the switch "Light theme" while the dark theme is active', () => {
    expect(labels('>light', context({ theme: 'dark' }))[0]).toBe('Light theme');
  });
});

describe('when filtering', () => {
  it('leaves out page actions when no page is open (spec "Page actions need a page")', () => {
    const listed = labels('>', context({ currentUuid: null }));
    expect(listed).not.toContain('Rename page');
    expect(listed).not.toContain('Move page');
    expect(listed).not.toContain('Delete page');
    expect(listed).toContain('Search pages');
  });

  it('lists page actions when a page is open', () => {
    const listed = labels('>');
    expect(listed).toEqual(expect.arrayContaining(['Rename page', 'Move page', 'Delete page']));
  });

  it('leaves out vault actions with no vault, and history it cannot follow', () => {
    const listed = ids('>', context({ hasVault: false, currentUuid: null, canGoBack: false }));
    expect(listed).not.toContain('folder.new');
    expect(listed).not.toContain('page.new');
    expect(listed).not.toContain('nav.back');
    expect(listed).toContain('vault.add');
  });
});

describe('recently run actions', () => {
  it('come first for an empty query, most recent first', () => {
    const listed = ids('>', context(), ['sparql.console', 'theme.toggle']);
    expect(listed.slice(0, 2)).toEqual(['sparql.console', 'theme.toggle']);
  });

  it('are remembered by runCommand', () => {
    const ctx = context();
    const theme = ACTIONS.find((a) => a.id === 'theme.toggle')!;
    const folder = ACTIONS.find((a) => a.id === 'folder.new')!;
    runCommand(theme, ctx, () => {});
    runCommand(folder, ctx, () => {});
    runCommand(theme, ctx, () => {});
    expect(recentActionIds()).toEqual(['theme.toggle', 'folder.new']);
    expect(commandResults('>', ctx).slice(0, 2).map((a) => a.id)).toEqual(['theme.toggle', 'folder.new']);
  });
});

describe('running an action', () => {
  it('closes search before running, so a dialog it opens stays open', () => {
    const order: string[] = [];
    const ctx = context({ newFolder: vi.fn(() => order.push('run')) });
    const [folder] = commandResults('>folder', ctx, ACTIONS, []);
    runCommand(folder, ctx, () => order.push('close'));
    expect(order).toEqual(['close', 'run']);
  });
});
