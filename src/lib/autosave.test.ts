import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createAutosave } from './autosave';

/** A promise plus the function that settles it. */
function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>((res) => {
    resolve = res;
  });
  return { promise, resolve };
}

describe('createAutosave', () => {
  let saved: string[];
  const record = async (key: string, value: string) => {
    saved.push(`${key}=${value}`);
  };

  beforeEach(() => {
    saved = [];
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('saves once, after the delay, with the latest value', async () => {
    const autosave = createAutosave(800, record);
    autosave.schedule('a', 'one', true);
    autosave.schedule('a', 'two', true);

    await vi.advanceTimersByTimeAsync(799);
    expect(saved).toEqual([]);
    await vi.advanceTimersByTimeAsync(1);
    expect(saved).toEqual(['a=two']);
  });

  it('saves the page being left when another page is scheduled before the delay', async () => {
    const autosave = createAutosave(800, record);
    autosave.schedule('a', 'edited', true);
    // Opening page b loads its content, which the editor schedules as not dirty.
    autosave.schedule('b', 'loaded', false);

    expect(saved).toEqual(['a=edited']);
    await vi.advanceTimersByTimeAsync(800);
    expect(saved).toEqual(['a=edited', 'b=loaded']);
  });

  it('does not write a page that was only opened when the next one is opened', async () => {
    const autosave = createAutosave(800, record);
    autosave.schedule('a', 'loaded', false);
    autosave.schedule('b', 'loaded too', false);

    await vi.advanceTimersByTimeAsync(800);
    expect(saved).toEqual(['b=loaded too']);
  });

  it('flush runs a waiting dirty save at once and waits for it', async () => {
    const gate = deferred();
    let done = false;
    const autosave = createAutosave(800, async (key: string, value: string) => {
      saved.push(`${key}=${value}`);
      await gate.promise;
      done = true;
    });
    autosave.schedule('a', 'edited', true);

    let flushed = false;
    const flushing = autosave.flush().then(() => {
      flushed = true;
    });
    await Promise.resolve();
    expect(saved).toEqual(['a=edited']);
    expect(flushed).toBe(false);

    gate.resolve();
    await flushing;
    expect(done).toBe(true);
    // The timer was taken off: nothing is saved twice.
    await vi.advanceTimersByTimeAsync(800);
    expect(saved).toEqual(['a=edited']);
  });

  it('flush waits for a save the timer already started', async () => {
    const gate = deferred();
    let done = false;
    const autosave = createAutosave(800, async () => {
      await gate.promise;
      done = true;
    });
    autosave.schedule('a', 'edited', true);
    await vi.advanceTimersByTimeAsync(800);

    const flushing = autosave.flush();
    gate.resolve();
    await flushing;
    expect(done).toBe(true);
  });

  it('flush drops a waiting save that holds no edits', async () => {
    const autosave = createAutosave(800, record);
    autosave.schedule('a', 'loaded', false);
    await autosave.flush();
    await vi.advanceTimersByTimeAsync(800);
    expect(saved).toEqual([]);
  });

  it('flush resolves when the save fails', async () => {
    const error = vi.spyOn(console, 'error').mockImplementation(() => {});
    const autosave = createAutosave(800, async () => {
      throw new Error('disk full');
    });
    autosave.schedule('a', 'edited', true);
    await expect(autosave.flush()).resolves.toBeUndefined();
    expect(error).toHaveBeenCalled();
    error.mockRestore();
  });

  it('cancel with a key leaves a save waiting under another key', async () => {
    const autosave = createAutosave(800, record);
    autosave.schedule('a', 'edited', true);
    autosave.cancel('b');
    await vi.advanceTimersByTimeAsync(800);
    expect(saved).toEqual(['a=edited']);

    autosave.schedule('a', 'again', true);
    autosave.cancel('a');
    await vi.advanceTimersByTimeAsync(800);
    expect(saved).toEqual(['a=edited']);
  });
});
