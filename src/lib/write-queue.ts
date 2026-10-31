/**
 * Run async jobs one at a time, in the order they were queued. Vault changes
 * (`page_write`, `page_create`, …) run off the main thread in the desktop
 * layer, so two in flight at once could land in the wrong order; queued, an
 * older save can never overwrite a newer one. A failed job does not stop the
 * ones behind it.
 */
export function serialQueue(): <T>(job: () => Promise<T>) => Promise<T> {
  let tail: Promise<unknown> = Promise.resolve();
  return <T>(job: () => Promise<T>): Promise<T> => {
    const run = tail.then(job, job);
    tail = run.catch(() => undefined);
    return run;
  };
}
