/**
 * A delayed save that can be forced early.
 *
 * The page editor saves a short while after the last edit. Leaving the page
 * before that delay ran out (another page, another vault, quit, window close)
 * used to drop the save, and the last keystrokes with it. Here a waiting save
 * is never dropped while it holds unsaved edits: `flush` runs it at once, and
 * scheduling a save under another key flushes the one waiting first.
 *
 * `dirty` says whether the value differs from what is stored. A waiting save
 * that is not dirty (the editor also schedules one when a page loads) is
 * simply replaced or dropped, as before.
 */
export interface Autosave<K> {
  /** Save `value` under `key` after the delay, replacing a waiting save for the same key. */
  schedule(key: K, value: string, dirty: boolean): void;
  /** Run a waiting dirty save now, then wait for every save still running. Never rejects. */
  flush(): Promise<void>;
  /** Drop the waiting save; with a key, only when it is waiting under that key. */
  cancel(key?: K): void;
}

export function createAutosave<K>(delayMs: number, save: (key: K, value: string) => Promise<void>): Autosave<K> {
  let waiting: { key: K; value: string; dirty: boolean; timer: ReturnType<typeof setTimeout> } | null = null;
  const running = new Set<Promise<void>>();

  function run(key: K, value: string): void {
    const job = (async () => save(key, value))()
      .catch((e) => console.error('Autosave failed:', e))
      .finally(() => running.delete(job));
    running.add(job);
  }

  /** Take the waiting save off the timer; run it when it holds unsaved edits or `always`. */
  function release(always: boolean): void {
    if (!waiting) return;
    const { key, value, dirty, timer } = waiting;
    clearTimeout(timer);
    waiting = null;
    if (dirty || always) run(key, value);
  }

  return {
    schedule(key, value, dirty) {
      // A save waiting under another key belongs to the page being left.
      if (waiting && waiting.key !== key) release(false);
      if (waiting) clearTimeout(waiting.timer);
      waiting = { key, value, dirty, timer: setTimeout(() => release(true), delayMs) };
    },
    async flush() {
      release(false);
      // Another save may start while these settle.
      while (running.size > 0) await Promise.all([...running]);
    },
    cancel(key) {
      if (!waiting || (key !== undefined && waiting.key !== key)) return;
      clearTimeout(waiting.timer);
      waiting = null;
    },
  };
}
