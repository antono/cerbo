/**
 * State of the "Similar notes" sidebar section (change `add-semantic-desktop-ui`).
 *
 * Similar notes come from `similar_get`, which may need embeddings the vault
 * does not have yet, the one-time model download, or an index held by the CLI.
 * None of these block the page: the section shows a state instead of the list.
 * Every transition is a pure function, so the store only assigns the result.
 */

/** What the section shows. */
export type SimilarStatus =
  | 'idle' // no page open
  | 'loading' // `similar_get` is running for `uuid`
  | 'ready' // `notes` is the list for `uuid` (may be empty)
  | 'disabled' // the vault has no embeddings: offer "Enable"
  | 'enabling' // `embeddings_refresh` is running
  | 'modelUnavailable' // the model cannot be obtained: offer "Retry"
  | 'busy' // another Cerbo process holds the index: offer "Retry"
  | 'error';

/** Mirrors `SearchHit` in cerbo-core: `score` is the raw cosine, 0..1. */
export interface SimilarNote {
  uuid: string;
  title: string;
  score: number;
}

/** Mirrors `SimilarNotes` in `src-tauri/src/semantic.rs`. */
export interface SimilarNotesResponse {
  uuid: string;
  notes: SimilarNote[];
}

/** Mirrors `SemanticProgress`, the payload of {@link PROGRESS_EVENT}. */
export interface SemanticProgress {
  phase: 'download' | 'embed' | 'similarity';
  done: number;
  total: number;
}

/**
 * Mirrors `SemanticCommandError` in `src-tauri/src/semantic.rs`: `kind` is
 * e.g. 'indexBusy', 'noEmbeddings', 'modelUnavailable' or 'unavailable'; every
 * kind but 'indexBusy' and 'noEmbeddings' carries a message.
 */
export interface SemanticCommandError {
  kind: string;
  message?: string;
}

/** Emitted by `embeddings_refresh` and `similar_get` while they work. */
export const PROGRESS_EVENT = 'semantic://progress';

export const SIMILAR_BUSY_MESSAGE =
  'Similar notes are temporarily unavailable: another Cerbo process is using the index.';

export interface SimilarState {
  /** The page the section is for: the last one requested. */
  uuid: string | null;
  status: SimilarStatus;
  notes: SimilarNote[];
  message: string;
  /** The latest progress step while loading or enabling, else null. */
  progress: SemanticProgress | null;
}

export const initialSimilarState: SimilarState = {
  uuid: null,
  status: 'idle',
  notes: [],
  message: '',
  progress: null,
};

/**
 * A request for page `uuid` starts. While embeddings are being enabled the
 * section keeps showing that progress; the list loads once enabling ends.
 */
export function similarRequested(state: SimilarState, uuid: string): SimilarState {
  if (state.status === 'enabling') return { ...state, uuid };
  return { uuid, status: 'loading', notes: [], message: '', progress: null };
}

/**
 * Whether an answer for `uuid` may update the section: only while that page
 * is still open and still the one the section asked for (design D2).
 */
export function isCurrent(state: SimilarState, openUuid: string | null, uuid: string): boolean {
  return uuid === openUuid && uuid === state.uuid && state.status !== 'enabling';
}

/** `similar_get` answered; a stale answer leaves the state unchanged. */
export function similarLoaded(
  state: SimilarState,
  openUuid: string | null,
  response: SimilarNotesResponse,
): SimilarState {
  if (!isCurrent(state, openUuid, response.uuid)) return state;
  return { uuid: response.uuid, status: 'ready', notes: response.notes, message: '', progress: null };
}

/** `similar_get` for `uuid` failed; a stale failure leaves the state unchanged. */
export function similarFailed(
  state: SimilarState,
  openUuid: string | null,
  uuid: string,
  error: unknown,
): SimilarState {
  if (!isCurrent(state, openUuid, uuid)) return state;
  return { ...failure(error), uuid, notes: [], progress: null };
}

/** "Enable" (or "Retry") was clicked: embeddings are being computed. */
export function enableStarted(state: SimilarState): SimilarState {
  return { ...state, status: 'enabling', notes: [], message: '', progress: null };
}

/**
 * `embeddings_refresh` finished: the list of the page open now loads, without
 * reopening it. With no page open the section goes idle.
 */
export function enableSucceeded(state: SimilarState, openUuid: string | null): SimilarState {
  if (!openUuid) return { ...initialSimilarState };
  return similarRequested({ ...state, status: 'idle' }, openUuid);
}

/** `embeddings_refresh` failed: show why, with "Retry" where it can help. */
export function enableFailed(state: SimilarState, error: unknown): SimilarState {
  return { ...state, ...failure(error), notes: [], progress: null };
}

/** A progress event arrived; it only shows while something is running. */
export function progressReceived(state: SimilarState, progress: SemanticProgress): SimilarState {
  if (state.status !== 'loading' && state.status !== 'enabling') return state;
  return { ...state, progress };
}

/**
 * Whether `similar_get` skipped its work because a newer request came in while
 * it waited for the index (the desktop runs index work one job at a time). The
 * newer request answers; this one must not touch the section.
 */
export function isSuperseded(error: unknown): boolean {
  return typeof error === 'object' && error !== null && (error as SemanticCommandError).kind === 'superseded';
}

/** Map a rejected semantic command to the section state and its message. */
export function failure(error: unknown): { status: SimilarStatus; message: string } {
  if (typeof error === 'object' && error !== null && 'kind' in error) {
    const e = error as SemanticCommandError;
    switch (e.kind) {
      case 'noEmbeddings':
        return { status: 'disabled', message: '' };
      case 'indexBusy':
        return { status: 'busy', message: SIMILAR_BUSY_MESSAGE };
      case 'modelUnavailable':
        return { status: 'modelUnavailable', message: e.message ?? '' };
      default:
        return { status: 'error', message: `Similar notes are unavailable: ${e.message ?? e.kind}` };
    }
  }
  return { status: 'error', message: `Similar notes are unavailable: ${String(error)}` };
}

/** A cosine score as a whole percentage, e.g. 0.137 → "14%". */
export function scorePercent(score: number): string {
  const clamped = Math.min(1, Math.max(0, score));
  return `${Math.round(clamped * 100)}%`;
}

/** One line describing a progress step, e.g. "Embedding pages 37 / 137". */
export function progressLabel(progress: SemanticProgress | null): string {
  if (!progress) return 'Starting…';
  const count = progress.total > 0 ? ` ${progress.done} / ${progress.total}` : '';
  switch (progress.phase) {
    case 'download':
      return `Downloading the embedding model (one time)${count}`;
    case 'embed':
      return `Embedding pages${count}`;
    case 'similarity':
      return 'Linking similar pages…';
  }
}

/** Progress as a 0..1 fraction for a bar, or null when it has no count. */
export function progressFraction(progress: SemanticProgress | null): number | null {
  if (!progress || progress.total <= 0) return null;
  return Math.min(1, progress.done / progress.total);
}
