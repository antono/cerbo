import { describe, expect, it } from 'vitest';
import { serialQueue } from './write-queue';

/** A promise plus the function that settles it. */
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

describe('serialQueue', () => {
  it('starts a job only after the one before it settled', async () => {
    const queue = serialQueue();
    const order: string[] = [];
    const first = deferred<number>();

    const a = queue(() => {
      order.push('a start');
      return first.promise;
    });
    const b = queue(async () => {
      order.push('b start');
      return 137;
    });

    await Promise.resolve();
    expect(order).toEqual(['a start']);

    first.resolve(1);
    expect(await a).toBe(1);
    expect(await b).toBe(137);
    expect(order).toEqual(['a start', 'b start']);
  });

  it('keeps going after a failed job and still reports its error', async () => {
    const queue = serialQueue();
    const failed = queue(() => Promise.reject(new Error('disk full')));
    const next = queue(async () => 137);

    await expect(failed).rejects.toThrow('disk full');
    expect(await next).toBe(137);
  });
});
