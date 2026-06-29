import { expect, it, vi } from 'vitest';
import { GapBuffer } from './gap-buffer';

const enc = (...ids: number[]) => new Uint8Array(ids);
const dec = (p: Uint8Array) => [...p];

function makeFakePartition(known: Set<number>) {
  return (payload: Uint8Array) => {
    const ids = dec(payload).toSorted((a, b) => a - b);
    const ready: number[] = [];
    const blocked: number[] = [];
    const seen = new Set(known);
    for (const id of ids) {
      if (id === 0 || seen.has(id - 1)) {
        ready.push(id);
        seen.add(id);
      } else {
        blocked.push(id);
      }
    }
    return { ready: enc(...ready), blocked: enc(...blocked) };
  };
}

it('applies ready changesets immediately', () => {
  const known = new Set<number>();
  const applied: number[][] = [];
  const buf = new GapBuffer({
    partition: makeFakePartition(known),
    apply: (ready) => {
      for (const id of dec(ready)) known.add(id);
      applied.push(dec(ready));
    },
    onStuck: vi.fn(),
  });
  buf.ingest(enc(0, 1));
  expect(applied).toEqual([[0, 1]]);
});

it('buffers an out-of-order changeset then applies it when the parent arrives', () => {
  const known = new Set<number>();
  const applied: number[][] = [];
  const buf = new GapBuffer({
    partition: makeFakePartition(known),
    apply: (ready) => {
      for (const id of dec(ready)) known.add(id);
      applied.push(dec(ready));
    },
    onStuck: vi.fn(),
  });
  buf.ingest(enc(2));
  expect(applied).toEqual([]);
  buf.ingest(enc(0, 1));
  expect(applied.flat()).toEqual([0, 1, 2]);
});

it('calls onStuck after threshold with no progress, then clears the buffer', () => {
  const known = new Set<number>();
  const onStuck = vi.fn();
  const buf = new GapBuffer({
    partition: makeFakePartition(known),
    apply: vi.fn(),
    onStuck,
    stuckThreshold: 2,
  });
  buf.ingest(enc(5));
  buf.ingest(enc(6));
  expect(onStuck).toHaveBeenCalledTimes(1);
  const stuckIds = (onStuck.mock.calls[0][0] as Uint8Array[]).flatMap(dec).toSorted((a, b) => a - b);
  expect(stuckIds).toEqual([5, 6]);
});
