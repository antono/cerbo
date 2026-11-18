/**
 * Hotkey utility for cross-platform keyboard event normalization.
 * Maps Ctrl (Linux/Windows) to Meta/Cmd (Mac).
 */

export const isMac = typeof window !== 'undefined' && /Mac|iPod|iPhone|iPad/.test(navigator.userAgent);

/**
 * Checks if the given keyboard event matches the primary modifier and key.
 * @param e - KeyboardEvent
 * @param key - The key name (case-insensitive)
 */
export function isModKey(e: KeyboardEvent, key: string): boolean {
  const mod = isMac ? e.metaKey : e.ctrlKey;
  return mod && e.key.toLowerCase() === key.toLowerCase();
}

/** `Ctrl+Shift+E` (`Cmd+Shift+E` on Mac): open or close the SPARQL console. */
export function isSparqlConsoleKey(e: KeyboardEvent): boolean {
  return isModKey(e, 'e') && e.shiftKey && !e.altKey;
}

/** Global shortcuts stay off while a confirmation modal owns the keyboard. */
export function confirmationOpen(state: { showExitPrompt: boolean; confirmDeleteUuid: string | null }): boolean {
  return state.showExitPrompt || state.confirmDeleteUuid !== null;
}

/** The transient-element flags `anyDialogOpen` reads. */
export interface TransientState {
  showSearch: boolean;
  showExitPrompt: boolean;
  showNewPageForm: boolean;
  showVaultSelector: boolean;
  showHelp: boolean;
  toolbarMenu: string | null;
  renameUuid: string | null;
  confirmDeleteUuid: string | null;
  showSparqlConsole: boolean;
  moveUuid: string | null;
  renameFolderPath: string | null;
  showNewFolder: boolean;
}

/**
 * Any dialog, search, the vault selector or a toolbar menu is open
 * (confirmations included). While true, only `overDialogs` hotkeys run, and
 * they close it before opening their own element (one transient element).
 */
export function anyDialogOpen(s: TransientState): boolean {
  return (
    s.showSearch ||
    s.showExitPrompt ||
    s.showNewPageForm ||
    s.showVaultSelector ||
    s.showHelp ||
    s.toolbarMenu !== null ||
    s.renameUuid !== null ||
    s.confirmDeleteUuid !== null ||
    s.showSparqlConsole ||
    s.moveUuid !== null ||
    s.renameFolderPath !== null ||
    s.showNewFolder
  );
}

/**
 * Checks if the event target is an input-like element (input, textarea, select, contenteditable).
 */
export function isInputFocused(): boolean {
  const active = document.activeElement;
  if (!active) return false;
  
  const tag = active.tagName.toLowerCase();
  return (
    tag === 'input' || 
    tag === 'textarea' || 
    tag === 'select' || 
    (active as HTMLElement).isContentEditable
  );
}
