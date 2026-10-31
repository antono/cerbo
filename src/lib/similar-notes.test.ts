import { describe, expect, it } from 'vitest';
import {
  enableFailed,
  enableStarted,
  enableSucceeded,
  failure,
  initialSimilarState,
  isCurrent,
  isSuperseded,
  progressFraction,
  progressLabel,
  progressReceived,
  scorePercent,
  SIMILAR_BUSY_MESSAGE,
  similarFailed,
  similarLoaded,
  similarRequested,
  type SimilarState,
} from './similar-notes';

const A = '00000000-0000-4000-8000-000000000137';
const B = '00000000-0000-4000-8000-000000000138';
const note = { uuid: B, title: 'Berlin trip', score: 0.137 };

describe('similar notes state', () => {
  it('loads and shows the list for the open page', () => {
    const loading = similarRequested(initialSimilarState, A);
    expect(loading).toMatchObject({ uuid: A, status: 'loading', notes: [] });
    const ready = similarLoaded(loading, A, { uuid: A, notes: [note] });
    expect(ready).toMatchObject({ uuid: A, status: 'ready', notes: [note] });
  });

  it('keeps an empty list as ready (empty-state message)', () => {
    const ready = similarLoaded(similarRequested(initialSimilarState, A), A, { uuid: A, notes: [] });
    expect(ready.status).toBe('ready');
    expect(ready.notes).toEqual([]);
  });

  it("drops page A's late answer once page B is open", () => {
    const forA = similarRequested(initialSimilarState, A);
    const forB = similarRequested(forA, B);
    // A answers after B was opened: ignored.
    expect(similarLoaded(forB, B, { uuid: A, notes: [note] })).toBe(forB);
    expect(similarFailed(forB, B, A, { kind: 'indexBusy' })).toBe(forB);
    // B's own answer is shown.
    expect(similarLoaded(forB, B, { uuid: B, notes: [] }).status).toBe('ready');
  });

  it('drops an answer for a page that is no longer open', () => {
    const forA = similarRequested(initialSimilarState, A);
    expect(isCurrent(forA, B, A)).toBe(false);
    expect(similarLoaded(forA, B, { uuid: A, notes: [note] })).toBe(forA);
  });

  it('offers Enable when the vault has no embeddings', () => {
    const s = similarFailed(similarRequested(initialSimilarState, A), A, A, { kind: 'noEmbeddings' });
    expect(s.status).toBe('disabled');
  });

  it('shows a non-blocking message when the index is busy', () => {
    const s = similarFailed(similarRequested(initialSimilarState, A), A, A, { kind: 'indexBusy' });
    expect(s).toMatchObject({ status: 'busy', message: SIMILAR_BUSY_MESSAGE });
  });

  it('enables embeddings with progress, then loads the open page', () => {
    const disabled: SimilarState = { ...initialSimilarState, uuid: A, status: 'disabled' };
    let s = enableStarted(disabled);
    expect(s).toMatchObject({ status: 'enabling', progress: null });

    s = progressReceived(s, { phase: 'download', done: 2, total: 5 });
    expect(s.progress).toEqual({ phase: 'download', done: 2, total: 5 });

    // Navigating while enabling keeps the progress but remembers the page.
    s = similarRequested(s, B);
    expect(s).toMatchObject({ uuid: B, status: 'enabling' });
    // A late similar_get answer cannot replace the progress.
    expect(similarLoaded(s, B, { uuid: B, notes: [note] })).toBe(s);

    s = enableSucceeded(s, B);
    expect(s).toMatchObject({ uuid: B, status: 'loading', progress: null });
    expect(similarLoaded(s, B, { uuid: B, notes: [note] }).notes).toEqual([note]);
  });

  it('goes idle after enabling when no page is open', () => {
    const s = enableSucceeded(enableStarted({ ...initialSimilarState, uuid: A }), null);
    expect(s).toEqual(initialSimilarState);
  });

  it('offers Retry when the model cannot be obtained', () => {
    const s = enableFailed(enableStarted({ ...initialSimilarState, uuid: A }), {
      kind: 'modelUnavailable',
      message: 'download failed',
    });
    expect(s).toMatchObject({ uuid: A, status: 'modelUnavailable', message: 'download failed' });
  });

  it('ignores progress when nothing is running', () => {
    const ready = similarLoaded(similarRequested(initialSimilarState, A), A, { uuid: A, notes: [] });
    expect(progressReceived(ready, { phase: 'embed', done: 1, total: 137 })).toBe(ready);
  });

  it('shows progress while a load re-embeds changed pages', () => {
    const s = progressReceived(similarRequested(initialSimilarState, A), {
      phase: 'embed',
      done: 37,
      total: 137,
    });
    expect(s.progress).toEqual({ phase: 'embed', done: 37, total: 137 });
  });
});

describe('failure', () => {
  it('names the reason for other errors', () => {
    expect(failure({ kind: 'unavailable', message: 'disk full' })).toEqual({
      status: 'error',
      message: 'Similar notes are unavailable: disk full',
    });
    expect(failure('No vault is open').message).toContain('No vault is open');
  });
});

describe('isSuperseded', () => {
  it('recognises only the superseded kind', () => {
    expect(isSuperseded({ kind: 'superseded' })).toBe(true);
    expect(isSuperseded({ kind: 'indexBusy' })).toBe(false);
    expect(isSuperseded('superseded')).toBe(false);
    expect(isSuperseded(null)).toBe(false);
  });
});

describe('formatting', () => {
  it('shows scores as whole percentages', () => {
    expect(scorePercent(0.137)).toBe('14%');
    expect(scorePercent(1)).toBe('100%');
    expect(scorePercent(-0.2)).toBe('0%');
  });

  it('describes each progress step', () => {
    expect(progressLabel(null)).toBe('Starting…');
    expect(progressLabel({ phase: 'download', done: 1, total: 5 })).toBe(
      'Downloading the embedding model (one time) 1 / 5',
    );
    expect(progressLabel({ phase: 'embed', done: 37, total: 137 })).toBe('Embedding pages 37 / 137');
    expect(progressLabel({ phase: 'similarity', done: 0, total: 0 })).toBe('Linking similar pages…');
  });

  it('has no bar fraction without a count', () => {
    expect(progressFraction({ phase: 'similarity', done: 0, total: 0 })).toBeNull();
    expect(progressFraction({ phase: 'embed', done: 137, total: 137 })).toBe(1);
  });
});
