import { describe, expect, it } from 'vitest';
import { backlinksFailure, INDEX_BUSY_MESSAGE } from './backlinks';

describe('backlinksFailure', () => {
  it('shows a loading state while the index builds', () => {
    expect(backlinksFailure({ kind: 'building' })).toEqual({ status: 'loading', message: '' });
  });

  it('shows a non-blocking message when the index is busy', () => {
    expect(backlinksFailure({ kind: 'indexBusy' })).toEqual({
      status: 'unavailable',
      message: INDEX_BUSY_MESSAGE,
    });
  });

  it('names the reason when the index cannot be used', () => {
    expect(backlinksFailure({ kind: 'unavailable', message: 'semantic index: disk full' })).toEqual({
      status: 'error',
      message: 'Backlinks are unavailable: semantic index: disk full',
    });
  });

  it('handles a plain string error', () => {
    expect(backlinksFailure('No vault is open').status).toBe('error');
    expect(backlinksFailure('No vault is open').message).toContain('No vault is open');
  });

  it('names the list in the messages of another panel list', () => {
    expect(backlinksFailure({ kind: 'indexBusy' }, 'Outgoing links').message).toBe(
      'Outgoing links are temporarily unavailable: another Cerbo process is using the index.',
    );
    expect(backlinksFailure({ kind: 'unavailable', message: 'x' }, 'Outgoing links').message).toBe(
      'Outgoing links are unavailable: x',
    );
  });
});
