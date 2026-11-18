/**
 * What the backlinks panel shows while `backlinks_get` has no list for it.
 *
 * Backlinks come from the vault's semantic index (cerbo-core), which may still
 * be building or be held by the CLI. Neither blocks the page: the panel shows a
 * loading state or a short message instead of the list.
 */

/** Panel state: the list, a loading state, or a non-blocking message. */
export type BacklinksStatus = 'ready' | 'loading' | 'unavailable' | 'error';

/** Mirrors `BacklinksError` in `src-tauri/src/index.rs`. */
export type BacklinksError =
  | { kind: 'building' }
  | { kind: 'indexBusy' }
  | { kind: 'unavailable'; message: string };

/** Emitted by the desktop once the opened vault's index is ready. */
export const INDEX_READY_EVENT = 'semantic-index-ready';

/** The message while another process holds the index; `what` names the list. */
export function indexBusyMessage(what = 'Backlinks'): string {
  return `${what} are temporarily unavailable: another Cerbo process is using the index.`;
}

export const INDEX_BUSY_MESSAGE = indexBusyMessage();

/**
 * Map a rejected `backlinks_get` (or `outgoing_links_get`, with `what` set to
 * its list's name) call to the panel state and its message.
 */
export function backlinksFailure(error: unknown, what = 'Backlinks'): { status: BacklinksStatus; message: string } {
  if (typeof error === 'object' && error !== null && 'kind' in error) {
    const e = error as BacklinksError;
    switch (e.kind) {
      case 'building':
        return { status: 'loading', message: '' };
      case 'indexBusy':
        return { status: 'unavailable', message: indexBusyMessage(what) };
      case 'unavailable':
        return { status: 'error', message: `${what} are unavailable: ${e.message}` };
    }
  }
  return { status: 'error', message: `${what} are unavailable: ${String(error)}` };
}
