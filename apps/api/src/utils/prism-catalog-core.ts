export const createTtlCache = <T>(opts: {
  load: () => Promise<T>;
  ttlMs: number;
  failureTtlMs?: number;
  onFailure?: (err: unknown) => void;
  now?: () => number;
}): (() => Promise<T>) => {
  const now = opts.now ?? (() => Date.now());
  let entry: { value: T; at: number } | null = null;
  let failure: { error: unknown; at: number } | null = null;
  return async () => {
    if (entry !== null && now() - entry.at < opts.ttlMs) return entry.value;
    if (failure !== null && opts.failureTtlMs !== undefined && now() - failure.at < opts.failureTtlMs) throw failure.error;
    try {
      const value = await opts.load();
      entry = { value, at: now() };
      failure = null;
      return value;
    } catch (err) {
      failure = { error: err, at: now() };
      opts.onFailure?.(err);
      throw err;
    }
  };
};
