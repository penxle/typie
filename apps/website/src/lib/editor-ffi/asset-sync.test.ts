import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createAssetSync } from './asset-sync';
import type { AssetStateEntry } from '$lib/sync/protocol';

const ready = (id: string): AssetStateEntry => ({
  id,
  state: 'ready',
  asset: { type: 'image', id, url: `https://cdn/${id}`, originalUrl: `https://cdn/${id}?raw`, width: 10, height: 20, placeholder: null },
});

const pending = (id: string): AssetStateEntry => ({ id, state: 'pending', meta: { kind: 'image', name: `${id}.png`, size: 100 } });

const missing = (id: string): AssetStateEntry => ({ id, state: 'missing' });

type Options = Parameters<typeof createAssetSync>[0];

const harness = (overrides: Partial<Options> = {}) => {
  const pulls: { requestId: string; ids: string[] }[] = [];
  const applied: AssetStateEntry[][] = [];
  const sync = createAssetSync({
    pull: (requestId, ids) => {
      pulls.push({ requestId, ids });
    },
    apply: (entries) => {
      applied.push(entries);
    },
    basePollMs: 1000,
    maxPollMs: 4000,
    debounceMs: 10,
    ...overrides,
  });

  const lastRequestId = () => pulls.at(-1)?.requestId ?? '';
  const idsOf = () => pulls.map(({ ids }) => ids);

  return { applied, idsOf, lastRequestId, pulls, sync };
};

describe('asset sync', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('pulls only ids with no known state and coalesces the burst into one request', () => {
    const { sync, pulls, idsOf, lastRequestId } = harness();

    sync.update(['a', 'b']);
    sync.update(['a', 'b', 'c']);
    vi.advanceTimersByTime(10);

    expect(idsOf()).toEqual([['a', 'b', 'c']]);

    sync.receive(lastRequestId(), [ready('a'), pending('b'), missing('c')], true);
    pulls.length = 0;

    sync.update(['a', 'b', 'c', 'd']);
    vi.advanceTimersByTime(10);

    expect(idsOf()).toEqual([['d']]);
  });

  it('does not re-pull an id whose response has not arrived yet', () => {
    const { sync, idsOf } = harness();

    sync.update(['a']);
    vi.advanceTimersByTime(10);
    sync.update(['a']);
    vi.advanceTimersByTime(10);

    expect(idsOf()).toEqual([['a']]);
  });

  it('discards entries that arrive under a superseded requestId', () => {
    const { sync, pulls, applied } = harness();

    sync.update(['a']);
    vi.advanceTimersByTime(10);
    const stale = pulls[0].requestId;

    sync.invalidate(['a']);
    vi.advanceTimersByTime(10);
    const fresh = pulls[1].requestId;

    expect(fresh).not.toBe(stale);

    sync.receive(fresh, [missing('a')], true);
    sync.receive(stale, [pending('a')], true);

    expect(applied).toEqual([[missing('a')]]);
  });

  it('clears the awaiting mark when the final frame carries no entry for a requested id', () => {
    const { sync, pulls, idsOf, lastRequestId } = harness();

    sync.update(['a']);
    vi.advanceTimersByTime(10);
    const requestId = lastRequestId();
    pulls.length = 0;

    sync.receive(requestId, [], true);

    sync.update(['a']);
    vi.advanceTimersByTime(10);

    expect(idsOf()).toEqual([['a']]);
  });

  it('accumulates a chunked response across frames sharing one requestId', () => {
    const { sync, applied, lastRequestId } = harness();

    sync.update(['a', 'b']);
    vi.advanceTimersByTime(10);
    const requestId = lastRequestId();

    sync.receive(requestId, [ready('a')], false);
    sync.receive(requestId, [pending('b')], true);

    expect(applied).toEqual([[ready('a')], [pending('b')]]);
  });

  it('invalidates only referenced ids', () => {
    const { sync, pulls, idsOf } = harness();

    sync.update(['a']);
    vi.advanceTimersByTime(10);
    pulls.length = 0;

    sync.invalidate(['a', 'gone']);
    vi.advanceTimersByTime(10);

    expect(idsOf()).toEqual([['a']]);

    pulls.length = 0;
    sync.invalidate(['gone']);
    vi.advanceTimersByTime(10);

    expect(pulls).toHaveLength(0);
  });

  it('drops queued ids that leave the reference set before the debounce fires', () => {
    const { sync, pulls, idsOf, lastRequestId } = harness();

    sync.update(['a', 'b']);
    vi.advanceTimersByTime(10);
    sync.receive(lastRequestId(), [missing('a'), missing('b')], true);
    pulls.length = 0;

    sync.invalidate(['a', 'b']);
    sync.update(['a']);
    vi.advanceTimersByTime(10);

    expect(idsOf()).toEqual([['a']]);
  });

  it('coalesces an invalidation storm into a single pull', () => {
    const { sync, pulls, idsOf, lastRequestId } = harness();

    sync.update(['a', 'b', 'c']);
    vi.advanceTimersByTime(10);
    sync.receive(lastRequestId(), [pending('a'), pending('b'), pending('c')], true);
    pulls.length = 0;

    sync.invalidate(['a']);
    sync.invalidate(['b']);
    sync.invalidate(['a', 'c']);
    vi.advanceTimersByTime(10);

    expect(idsOf()).toEqual([['a', 'b', 'c']]);
  });

  it('lets a completed asset win over later pending or missing frames', () => {
    const { sync, applied, pulls, lastRequestId } = harness();

    sync.update(['a']);
    vi.advanceTimersByTime(10);
    sync.receive(lastRequestId(), [pending('a')], true);

    sync.invalidate(['a']);
    vi.advanceTimersByTime(10);
    sync.receive(lastRequestId(), [ready('a')], true);

    sync.invalidate(['a']);
    vi.advanceTimersByTime(10);
    sync.receive(lastRequestId(), [missing('a')], true);

    expect(applied.flat()).toEqual([pending('a'), ready('a')]);

    pulls.length = 0;
    vi.advanceTimersByTime(60_000);

    expect(pulls).toHaveLength(0);
  });

  it('re-pulls a pending id after the poll interval and applies a silent expiry', () => {
    const { sync, applied, pulls, idsOf, lastRequestId } = harness();

    sync.update(['a']);
    vi.advanceTimersByTime(10);
    sync.receive(lastRequestId(), [pending('a')], true);
    pulls.length = 0;

    vi.advanceTimersByTime(1000);

    expect(idsOf()).toEqual([['a']]);

    sync.receive(lastRequestId(), [missing('a')], true);

    expect(applied.flat()).toEqual([pending('a'), missing('a')]);
  });

  it('keeps polling ids cached as missing so a dropped invalidation still converges', () => {
    const { sync, applied, pulls, idsOf, lastRequestId } = harness();

    sync.update(['a']);
    vi.advanceTimersByTime(10);
    sync.receive(lastRequestId(), [missing('a')], true);
    pulls.length = 0;

    vi.advanceTimersByTime(1000);

    expect(idsOf()).toEqual([['a']]);

    sync.receive(lastRequestId(), [ready('a')], true);

    expect(applied.flat()).toEqual([missing('a'), ready('a')]);
  });

  it('grows the poll interval up to the cap and resets it on a state change', () => {
    const { sync, pulls, lastRequestId } = harness();

    sync.update(['a']);
    vi.advanceTimersByTime(10);
    sync.receive(lastRequestId(), [missing('a')], true);
    pulls.length = 0;

    vi.advanceTimersByTime(1000);
    expect(pulls).toHaveLength(1);
    sync.receive(lastRequestId(), [missing('a')], true);

    vi.advanceTimersByTime(1000);
    expect(pulls).toHaveLength(1);
    vi.advanceTimersByTime(1000);
    expect(pulls).toHaveLength(2);
    sync.receive(lastRequestId(), [missing('a')], true);

    vi.advanceTimersByTime(4000);
    expect(pulls).toHaveLength(3);
    sync.receive(lastRequestId(), [missing('a')], true);

    vi.advanceTimersByTime(4000);
    expect(pulls).toHaveLength(4);

    sync.receive(lastRequestId(), [pending('a')], true);
    vi.advanceTimersByTime(1000);
    expect(pulls).toHaveLength(5);
  });

  it('resets the poll interval when the document comes back to the foreground', () => {
    const { sync, pulls, lastRequestId } = harness();

    sync.update(['a']);
    vi.advanceTimersByTime(10);
    sync.receive(lastRequestId(), [missing('a')], true);

    vi.advanceTimersByTime(1000);
    sync.receive(lastRequestId(), [missing('a')], true);
    vi.advanceTimersByTime(2000);
    sync.receive(lastRequestId(), [missing('a')], true);
    pulls.length = 0;

    sync.repullReferenced(['a']);
    expect(pulls).toHaveLength(1);

    vi.advanceTimersByTime(1000);
    expect(pulls).toHaveLength(2);
  });

  it('repulls every referenced non-ready id at once and skips ready ones', () => {
    const { sync, pulls, idsOf, lastRequestId } = harness();

    sync.update(['a', 'b', 'c']);
    vi.advanceTimersByTime(10);
    sync.receive(lastRequestId(), [ready('a'), pending('b'), missing('c')], true);
    pulls.length = 0;

    sync.repullReferenced(['a', 'b', 'c']);

    expect(idsOf()).toEqual([['b', 'c']]);
  });

  it('drops ids that left the reference set from polling and from responses', () => {
    const { sync, applied, pulls, idsOf, lastRequestId } = harness();

    sync.update(['a', 'b']);
    vi.advanceTimersByTime(10);
    const requestId = lastRequestId();
    sync.update(['a']);
    pulls.length = 0;
    applied.length = 0;

    sync.receive(requestId, [missing('a'), ready('b')], true);

    expect(applied.flat()).toEqual([missing('a')]);

    vi.advanceTimersByTime(1000);

    expect(idsOf()).toEqual([['a']]);
  });

  it('splits a pull over the wire limit into bounded requests', () => {
    const { sync, pulls } = harness();
    const ids = Array.from({ length: 250 }, (_, index) => `id-${index}`);

    sync.update(ids);
    vi.advanceTimersByTime(10);

    expect(pulls.map(({ ids: chunk }) => chunk.length)).toEqual([100, 100, 50]);
    expect(new Set(pulls.map(({ requestId }) => requestId)).size).toBe(3);
    expect(pulls.flatMap(({ ids: chunk }) => chunk)).toEqual(ids);
  });

  it('stops every timer and ignores further calls after dispose', () => {
    const { sync, pulls, lastRequestId } = harness();

    sync.update(['a']);
    vi.advanceTimersByTime(10);
    sync.receive(lastRequestId(), [missing('a')], true);
    pulls.length = 0;

    sync.dispose();

    expect(vi.getTimerCount()).toBe(0);

    sync.update(['a', 'b']);
    sync.invalidate(['a']);
    sync.repullReferenced(['a']);
    vi.advanceTimersByTime(60_000);

    expect(pulls).toHaveLength(0);
  });
});
