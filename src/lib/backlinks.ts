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

export const INDEX_BUSY_MESSAGE =
  'Backlinks are temporarily unavailable: another Cerbo process is using the index.';

/** Map a rejected `backlinks_get` call to the panel state and its message. */
export function backlinksFailure(error: unknown): { status: BacklinksStatus; message: string } {
  if (typeof error === 'object' && error !== null && 'kind' in error) {
    const e = error as BacklinksError;
    switch (e.kind) {
      case 'building':
        return { status: 'loading', message: '' };
      case 'indexBusy':
        return { status: 'unavailable', message: INDEX_BUSY_MESSAGE };
      case 'unavailable':
        return { status: 'error', message: `Backlinks are unavailable: ${e.message}` };
    }
  }
  return { status: 'error', message: `Backlinks are unavailable: ${String(error)}` };
}
