/**
 * Pure helpers behind the app-drawn title bar (frameless window).
 * Kept free of Tauri and Svelte imports so they run under vitest.
 */

/** The exit confirmation shared by `Ctrl+Q` and the close button (spec `application-lifecycle`). */
export const QUIT_PROMPT = {
  title: 'Quit Application?',
  message: 'Are you sure you want to quit?',
  confirmLabel: 'Quit',
} as const;

export interface ExitPromptState {
  showExitPrompt: boolean;
}

/**
 * Open the "Quit Application?" confirmation, exactly like `Ctrl+Q`:
 * close any other dialog first, then raise the exit prompt.
 */
export function requestQuit(state: ExitPromptState, closeDialogs: () => void): void {
  closeDialogs();
  state.showExitPrompt = true;
}

/** "Vault · Page" title parts; empty names are dropped. */
export function titleParts(vaultName: string | null | undefined, pageTitle: string | null | undefined): string[] {
  return [vaultName, pageTitle].map((part) => part?.trim() ?? '').filter((part) => part.length > 0);
}
