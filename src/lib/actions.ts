/**
 * The app action registry (design D1 of `unify-window-toolbar`).
 *
 * One list feeds the toolbar menus, command mode of page search, the global
 * hotkey handler and the keyboard shortcuts help, so they cannot drift apart.
 * Context-specific keys (tree `j`/`k`/`h`/`l`, preview scrolling `j`/`k`,
 * editor `i`/`Esc`) stay in their components.
 *
 * This module is pure: actions reach the app only through the `ActionContext`
 * they are given, so the registry is testable without Tauri or the store.
 */

import {
  ArrowDown,
  ArrowLeft,
  ArrowRight,
  ArrowUp,
  Command,
  Database,
  Eye,
  FilePlus,
  FolderInput,
  FolderOpen,
  FolderPlus,
  Keyboard,
  Link,
  LocateFixed,
  LogOut,
  Moon,
  PackagePlus,
  PanelLeft,
  PanelRight,
  Pencil,
  Search,
  Sun,
  TextCursorInput,
  Trash2,
} from 'lucide-svelte';
import { isMac } from './hotkeys';
import { RIGHT_PANELS, type RightPanelId } from './right-panels';

export type ActionIcon = typeof Search;

/** The app state an action looks at, and the operations it may call. */
export interface ActionContext {
  hasVault: boolean;
  /** UUID of the open page; null when none is open. */
  currentUuid: string | null;
  theme: 'light' | 'dark';
  editorTab: 'write' | 'preview';
  showLeftSidebar: boolean;
  showRightSidebar: boolean;
  /** The selected right panel tab. */
  rightPanelTab: RightPanelId;
  canGoBack: boolean;
  canGoForward: boolean;

  /** Open or close one dialog; opening closes every other transient element. */
  toggleDialog(dialog: 'search' | 'newPage' | 'vaultSelector' | 'help' | 'sparqlConsole'): void;
  /** Open page search with text already typed (`>` opens command mode). */
  openSearch(prefill: string): void;
  /** Close search when open, else open it with `prefill`. */
  toggleSearch(prefill: string): void;
  addVault(): void;
  toggleLeftSidebar(): void;
  toggleRightSidebar(): void;
  /** Show the right panel on tab `id`, or hide it when `id` is already selected. */
  useRightPanelTab(id: RightPanelId): void;
  toggleEditorMode(): void;
  setTheme(theme: 'light' | 'dark'): void;
  goBack(): void;
  goForward(): void;
  nextPage(): void;
  prevPage(): void;
  renamePage(): void;
  movePage(): void;
  deletePage(): void;
  copyPageLink(): void;
  revealPage(): void;
  newFolder(): void;
  quit(): void;
}

/**
 * A key combination. `mod` is `Ctrl`, or `Cmd` on Mac. Every modifier is
 * matched exactly: a missing flag means "not held".
 */
export interface Hotkey {
  /** `KeyboardEvent.key` values, matched case-insensitively. */
  key?: string | string[];
  /**
   * `KeyboardEvent.code`, for keys whose character differs between keyboard
   * layouts (`Backslash`). Takes the place of `key`.
   */
  code?: string;
  mod?: boolean;
  shift?: boolean;
  alt?: boolean;
  /**
   * Where the key fires: `any` (default) also in the editor and inputs,
   * `noInput` only when no input has focus, `preview` only in preview mode
   * with no input focused.
   */
  scope?: 'any' | 'noInput' | 'preview';
  /** Fires while a dialog is open (it toggles that dialog or its own). */
  overDialogs?: boolean;
  /** The main key as shown to the user, when its name is not enough (`J`). */
  displayKey?: string;
}

export interface Action {
  id: string;
  /** Name in menus, command mode and help; may depend on the app state. */
  label: string | ((ctx: ActionContext) => string);
  icon: ActionIcon;
  /** Icon for the current state, when it changes with it (theme, mode). */
  iconFor?: (ctx: ActionContext) => ActionIcon;
  hotkey?: Hotkey;
  /** Other words command mode matches, e.g. "dark" for the theme switch. */
  synonyms?: string[];
  /** False when the action does not apply right now; absent means always. */
  when?: (ctx: ActionContext) => boolean;
  run: (ctx: ActionContext) => void;
  /** Shown in red, last in its menu (Delete). */
  destructive?: boolean;
  /** Left out of command mode (the hotkey that opens command mode itself). */
  hideInCommands?: boolean;
  /** Help text in the shortcuts help, when it differs from the label. */
  helpLabel?: string;
}

const hasPage = (ctx: ActionContext) => ctx.currentUuid !== null;
const hasVault = (ctx: ActionContext) => ctx.hasVault;

export const ACTIONS: readonly Action[] = [
  // ── Search and commands ─────────────────────────────────────────────────
  {
    id: 'search.pages',
    label: 'Search pages',
    icon: Search,
    hotkey: { mod: true, key: 'p' },
    synonyms: ['find', 'open page', 'go to'],
    run: (ctx) => ctx.toggleSearch(''),
    helpLabel: 'Open page search',
  },
  {
    id: 'search.commands',
    label: 'Show all commands',
    icon: Command,
    hotkey: { mod: true, shift: true, key: 'p', overDialogs: true },
    synonyms: ['command palette'],
    run: (ctx) => ctx.toggleSearch('>'),
    hideInCommands: true,
    helpLabel: 'Search commands (command mode)',
  },

  // ── Vault ───────────────────────────────────────────────────────────────
  {
    id: 'vault.switch',
    label: 'Switch vault',
    icon: FolderOpen,
    hotkey: { mod: true, key: 'o' },
    synonyms: ['open vault', 'vault selector'],
    run: (ctx) => ctx.toggleDialog('vaultSelector'),
    helpLabel: 'Open vault selector',
  },
  {
    id: 'vault.add',
    label: 'Add vault',
    icon: PackagePlus,
    hotkey: { mod: true, shift: true, key: 'o' },
    synonyms: ['new vault', 'register vault'],
    run: (ctx) => ctx.addVault(),
  },

  // ── Panels and mode ─────────────────────────────────────────────────────
  {
    id: 'panel.left',
    label: (ctx) => (ctx.showLeftSidebar ? 'Hide page tree' : 'Show page tree'),
    icon: PanelLeft,
    hotkey: { mod: true, code: 'Backslash' },
    synonyms: ['page tree', 'sidebar', 'left panel', 'toggle'],
    run: (ctx) => ctx.toggleLeftSidebar(),
    helpLabel: 'Show or hide the page tree',
  },
  {
    id: 'panel.right',
    label: (ctx) => (ctx.showRightSidebar ? 'Hide right panel' : 'Show right panel'),
    icon: PanelRight,
    hotkey: { mod: true, shift: true, code: 'Backslash' },
    synonyms: ['right panel', 'backlinks', 'similar', 'attachments', 'files', 'toggle'],
    run: (ctx) => ctx.toggleRightSidebar(),
    helpLabel: 'Show or hide the right panel',
  },
  // One "Show <tab>" action per right panel tab, from the registry.
  ...RIGHT_PANELS.map(
    (panel): Action => ({
      id: `panel.right.${panel.id}`,
      label: `Show ${panel.label}`,
      icon: panel.icon,
      hotkey: panel.hotkey,
      synonyms: ['right panel', 'tab', panel.label.toLowerCase()],
      when: hasPage,
      run: (ctx) => ctx.useRightPanelTab(panel.id),
      helpLabel: `Show ${panel.label} (hide the right panel if already shown)`,
    }),
  ),
  {
    id: 'page.mode',
    // The eye switches to preview, the pencil to edit (spec
    // `desktop-window-toolbar`, "Edit and preview toggle").
    label: (ctx) => (ctx.editorTab === 'write' ? 'Switch to preview' : 'Switch to edit'),
    icon: Eye,
    iconFor: (ctx) => (ctx.editorTab === 'write' ? Eye : Pencil),
    hotkey: { mod: true, key: 'e' },
    synonyms: ['edit', 'preview', 'read', 'mode', 'toggle'],
    when: hasPage,
    run: (ctx) => ctx.toggleEditorMode(),
    helpLabel: 'Switch between edit and preview',
  },

  // ── Navigation ──────────────────────────────────────────────────────────
  {
    id: 'nav.back',
    label: 'Go back',
    icon: ArrowLeft,
    hotkey: { alt: true, key: ['ArrowLeft', 'Left'], scope: 'noInput', displayKey: '←' },
    synonyms: ['history', 'previous'],
    when: (ctx) => hasPage(ctx) && ctx.canGoBack,
    run: (ctx) => ctx.goBack(),
    helpLabel: 'Go back in history',
  },
  {
    id: 'nav.forward',
    label: 'Go forward',
    icon: ArrowRight,
    hotkey: { alt: true, key: ['ArrowRight', 'Right'], scope: 'noInput', displayKey: '→' },
    synonyms: ['history', 'next'],
    when: (ctx) => hasPage(ctx) && ctx.canGoForward,
    run: (ctx) => ctx.goForward(),
    helpLabel: 'Go forward in history',
  },
  {
    id: 'nav.next',
    label: 'Next page',
    icon: ArrowDown,
    // Upper-case J: Shift is held. Lower-case j scrolls the preview.
    hotkey: { shift: true, key: 'j', scope: 'preview', displayKey: 'J' },
    synonyms: ['down'],
    when: hasPage,
    run: (ctx) => ctx.nextPage(),
    helpLabel: 'Next page (Vim-style)',
  },
  {
    id: 'nav.prev',
    label: 'Previous page',
    icon: ArrowUp,
    hotkey: { shift: true, key: 'k', scope: 'preview', displayKey: 'K' },
    synonyms: ['up'],
    when: hasPage,
    run: (ctx) => ctx.prevPage(),
    helpLabel: 'Previous page (Vim-style)',
  },

  // ── Page ────────────────────────────────────────────────────────────────
  {
    id: 'page.new',
    label: 'New page',
    icon: FilePlus,
    hotkey: { mod: true, key: 'n', overDialogs: true },
    synonyms: ['create page', 'add page'],
    when: hasVault,
    run: (ctx) => ctx.toggleDialog('newPage'),
    helpLabel: 'Create new page',
  },
  {
    id: 'page.rename',
    label: 'Rename page',
    icon: TextCursorInput,
    hotkey: { key: 'r', scope: 'preview' },
    synonyms: ['title'],
    when: hasPage,
    run: (ctx) => ctx.renamePage(),
    helpLabel: 'Rename current page',
  },
  {
    id: 'page.move',
    label: 'Move page',
    icon: FolderInput,
    hotkey: { key: 'm', scope: 'preview' },
    synonyms: ['folder', 'path'],
    when: hasPage,
    run: (ctx) => ctx.movePage(),
    helpLabel: 'Move current page to a folder',
  },
  {
    id: 'page.copyLink',
    label: 'Copy link',
    icon: Link,
    synonyms: ['copy page link', 'clipboard', 'reference'],
    when: hasPage,
    run: (ctx) => ctx.copyPageLink(),
  },
  {
    id: 'page.reveal',
    label: 'Reveal in tree',
    icon: LocateFixed,
    synonyms: ['show in tree', 'locate', 'sidebar'],
    when: hasPage,
    run: (ctx) => ctx.revealPage(),
  },
  {
    id: 'page.delete',
    label: 'Delete page',
    icon: Trash2,
    hotkey: { key: ['Delete', 'Backspace'], scope: 'preview', displayKey: 'Del' },
    synonyms: ['remove', 'trash'],
    when: hasPage,
    run: (ctx) => ctx.deletePage(),
    destructive: true,
    helpLabel: 'Delete current page',
  },
  {
    id: 'folder.new',
    label: 'New folder',
    icon: FolderPlus,
    synonyms: ['create folder', 'add folder', 'directory'],
    when: hasVault,
    run: (ctx) => ctx.newFolder(),
  },

  // ── App ─────────────────────────────────────────────────────────────────
  {
    id: 'theme.toggle',
    // Named after the theme it switches to (spec `theme-toggle-component`).
    label: (ctx) => (ctx.theme === 'dark' ? 'Light theme' : 'Dark theme'),
    icon: Moon,
    iconFor: (ctx) => (ctx.theme === 'dark' ? Sun : Moon),
    hotkey: { mod: true, key: 't', overDialogs: true },
    synonyms: ['theme', 'dark', 'light', 'appearance', 'color'],
    run: (ctx) => ctx.setTheme(ctx.theme === 'dark' ? 'light' : 'dark'),
    helpLabel: 'Toggle light/dark theme',
  },
  {
    id: 'sparql.console',
    label: 'SPARQL console',
    icon: Database,
    hotkey: { mod: true, shift: true, key: 'e', overDialogs: true },
    synonyms: ['query', 'graph', 'rdf'],
    run: (ctx) => ctx.toggleDialog('sparqlConsole'),
  },
  {
    id: 'help.shortcuts',
    label: 'Keyboard shortcuts',
    icon: Keyboard,
    hotkey: { key: 'F1', overDialogs: true },
    synonyms: ['help', 'hotkeys', 'keys'],
    run: (ctx) => ctx.toggleDialog('help'),
    helpLabel: 'Show this help',
  },
  {
    id: 'app.quit',
    label: 'Quit',
    icon: LogOut,
    hotkey: { mod: true, key: 'q' },
    synonyms: ['exit', 'close', 'quit cerbo'],
    run: (ctx) => ctx.quit(),
    helpLabel: 'Quit application',
  },
];

/** Entries of the toolbar's page menu, Delete last (spec "Page path and page menu"). */
export const PAGE_MENU: readonly string[] = ['page.rename', 'page.move', 'page.copyLink', 'page.reveal', 'page.delete'];

/** Entries of the toolbar's ⋯ menu (spec "Overflow menu"). */
export const OVERFLOW_MENU: readonly string[] = ['theme.toggle', 'sparql.console', 'folder.new', 'help.shortcuts', 'app.quit'];

/** Global shortcuts listed in the help, in this order. */
export const HELP_ACTIONS: readonly string[] = [
  'search.pages',
  'search.commands',
  'page.new',
  'page.mode',
  'panel.left',
  'panel.right',
  ...RIGHT_PANELS.map((p) => `panel.right.${p.id}`),
  'theme.toggle',
  'vault.switch',
  'vault.add',
  'sparql.console',
  'app.quit',
  'help.shortcuts',
];

/** Navigation shortcuts listed in the help, in this order. */
export const HELP_NAVIGATION: readonly string[] = ['nav.back', 'nav.forward', 'nav.next', 'nav.prev'];

export function actionById(id: string, actions: readonly Action[] = ACTIONS): Action {
  const action = actions.find((a) => a.id === id);
  if (!action) throw new Error(`Unknown action: ${id}`);
  return action;
}

export function actionLabel(action: Action, ctx: ActionContext): string {
  return typeof action.label === 'function' ? action.label(ctx) : action.label;
}

export function actionIcon(action: Action, ctx: ActionContext): ActionIcon {
  return action.iconFor ? action.iconFor(ctx) : action.icon;
}

/** True when the action applies to the current state. */
export function actionAvailable(action: Action, ctx: ActionContext): boolean {
  return action.when ? action.when(ctx) : true;
}

/** Actions to offer right now, in registry order. */
export function availableActions(ctx: ActionContext, actions: readonly Action[] = ACTIONS): Action[] {
  return actions.filter((a) => actionAvailable(a, ctx));
}

// ── Hotkeys ───────────────────────────────────────────────────────────────────

const KEY_NAMES: Record<string, string> = {
  arrowleft: '←',
  arrowright: '→',
  arrowup: '↑',
  arrowdown: '↓',
  delete: 'Del',
  backslash: '\\',
  escape: 'Esc',
};

/** The keys of `hotkey` as shown to the user, e.g. `['Ctrl', 'Shift', '\\']`. */
export function hotkeyKeys(hotkey: Hotkey, mac = isMac): string[] {
  const keys: string[] = [];
  if (hotkey.mod) keys.push(mac ? '⌘' : 'Ctrl');
  if (hotkey.alt) keys.push(mac ? '⌥' : 'Alt');
  // `J` already says Shift; spell it out only when the key does not.
  if (hotkey.shift && !hotkey.displayKey) keys.push('Shift');
  if (hotkey.displayKey) {
    keys.push(hotkey.displayKey);
    return keys;
  }
  const digit = /^Digit(\d)$/.exec(hotkey.code ?? '');
  if (digit) {
    keys.push(digit[1]);
    return keys;
  }
  const main = hotkey.code ?? (Array.isArray(hotkey.key) ? hotkey.key[0] : hotkey.key) ?? '';
  const named = KEY_NAMES[main.toLowerCase()];
  // Ctrl+P reads as `P`; a plain key keeps its case, since `J` means Shift+j.
  const upper = main.length === 1 && (hotkey.mod || hotkey.alt);
  keys.push(named ?? (upper ? main.toUpperCase() : main));
  return keys;
}

/** `Ctrl+Shift+\` (`⌘+Shift+\` on Mac), for tooltips and command mode. */
export function hotkeyText(hotkey: Hotkey, mac = isMac): string {
  return hotkeyKeys(hotkey, mac).join('+');
}

/** One string per key the hotkey accepts, e.g. `mod+shift:code:Backslash`. */
export function hotkeySignatures(hotkey: Hotkey): string[] {
  const mods = `${hotkey.mod ? 'mod+' : ''}${hotkey.alt ? 'alt+' : ''}${hotkey.shift ? 'shift+' : ''}`;
  if (hotkey.code) return [`${mods}code:${hotkey.code}`];
  const keys = Array.isArray(hotkey.key) ? hotkey.key : hotkey.key ? [hotkey.key] : [];
  return keys.map((k) => `${mods}key:${k.toLowerCase()}`);
}

/** True when `e` is the key combination `hotkey` names. */
export function matchesHotkey(e: KeyboardEvent, hotkey: Hotkey, mac = isMac): boolean {
  const mod = mac ? e.metaKey : e.ctrlKey;
  if (!!hotkey.mod !== mod) return false;
  if (!!hotkey.shift !== e.shiftKey) return false;
  if (!!hotkey.alt !== e.altKey) return false;
  // Layout-independent keys: `Ctrl+Shift+\` reports `|` as its key on a US
  // layout and other characters elsewhere, but always the same code.
  if (hotkey.code) return e.code === hotkey.code;
  const keys = Array.isArray(hotkey.key) ? hotkey.key : hotkey.key ? [hotkey.key] : [];
  const pressed = pressedKey(e);
  return keys.some((k) => k.toLowerCase() === pressed);
}

const LATIN_LETTER = /^[a-z]$/i;
const ANY_LETTER = /^\p{L}$/u;

/**
 * The pressed key, lower-cased, for matching letter hotkeys. On a non-Latin
 * layout (Russian, Greek…) `e.key` is the local letter (`з` for the P key),
 * so a letter that is not Latin falls back to the physical key: `KeyP` -> `p`.
 * A Latin `e.key` wins, so Dvorak and AZERTY keep their own letters.
 */
export function pressedKey(e: KeyboardEvent): string {
  const key = e.key ?? '';
  if (ANY_LETTER.test(key) && !LATIN_LETTER.test(key)) {
    const m = /^Key([A-Z])$/.exec(e.code ?? '');
    if (m) return m[1].toLowerCase();
  }
  return key.toLowerCase();
}

/** Where a key event happens, as far as the global handler cares. */
export interface KeyContext {
  /** A modal dialog, search or the vault selector is open. */
  dialogOpen: boolean;
  /** An input, textarea or contenteditable element has focus. */
  inputFocused: boolean;
  editorTab: 'write' | 'preview';
}

/** The registry action whose hotkey `e` is in this context, if any. */
export function actionForKey(
  e: KeyboardEvent,
  where: KeyContext,
  actions: readonly Action[] = ACTIONS,
  mac = isMac,
): Action | null {
  for (const action of actions) {
    const hotkey = action.hotkey;
    if (!hotkey || !matchesHotkey(e, hotkey, mac)) continue;
    if (where.dialogOpen && !hotkey.overDialogs) continue;
    const scope = hotkey.scope ?? 'any';
    if (scope === 'noInput' && where.inputFocused) continue;
    if (scope === 'preview' && (where.inputFocused || where.editorTab !== 'preview')) continue;
    return action;
  }
  return null;
}

/**
 * Handle a global key event with the registry: run the matching action and
 * report whether one matched. A matched action that does not apply right now
 * still eats the key, so the webview's own binding (e.g. `Ctrl+E`) never runs.
 */
export function dispatchHotkey(
  e: KeyboardEvent,
  where: KeyContext,
  ctx: ActionContext,
  actions: readonly Action[] = ACTIONS,
  mac = isMac,
): Action | null {
  const action = actionForKey(e, where, actions, mac);
  if (!action) return null;
  e.preventDefault();
  if (actionAvailable(action, ctx)) action.run(ctx);
  return action;
}
