import { describe, expect, it } from 'vitest';
import { GlCanvasRecycler } from './gl-canvas-recycler';

type Item = { id: number; lost: boolean };

const setup = (cap: number) => {
  const disposed: Item[] = [];
  const recycler = new GlCanvasRecycler<Item>(cap, {
    isLost: (item) => item.lost,
    dispose: (item) => {
      disposed.push(item);
    },
  });
  let nextId = 0;
  const make = (lost = false): Item => ({ id: nextId++, lost });
  return { recycler, disposed, make };
};

describe('gl-canvas-recycler', () => {
  it('miss: acquire on an empty pool returns undefined', () => {
    const { recycler } = setup(4);
    expect(recycler.acquire()).toBeUndefined();
    expect(recycler.size()).toBe(0);
  });

  it('hit: a parked canvas is returned and removed from the pool', () => {
    const { recycler, make } = setup(4);
    const a = make();
    expect(recycler.park(a)).toBe('pooled');
    expect(recycler.size()).toBe(1);
    expect(recycler.acquire()).toBe(a);
    expect(recycler.size()).toBe(0);
  });

  it('reuses the most-recently-parked canvas first (warm LIFO)', () => {
    const { recycler, make } = setup(4);
    const a = make();
    const b = make();
    recycler.park(a);
    recycler.park(b);
    expect(recycler.acquire()).toBe(b);
    expect(recycler.acquire()).toBe(a);
  });

  it('evict: overflow past cap disposes the oldest', () => {
    const { recycler, disposed, make } = setup(2);
    const a = make();
    const b = make();
    const c = make();
    recycler.park(a);
    recycler.park(b);
    recycler.park(c); // overflow → oldest (a) evicted
    expect(disposed).toEqual([a]);
    expect(recycler.size()).toBe(2);
    expect(recycler.acquire()).toBe(c);
    expect(recycler.acquire()).toBe(b);
  });

  it('lost-at-park: a lost canvas is not pooled and reports "lost"', () => {
    const { recycler, disposed, make } = setup(4);
    const a = make(true);
    expect(recycler.park(a)).toBe('lost');
    expect(recycler.size()).toBe(0);
    expect(disposed).toEqual([]); // caller handles the lost canvas via its own path
  });

  it('lost-eviction: a canvas lost while pooled is skipped and disposed on acquire', () => {
    const { recycler, disposed, make } = setup(4);
    const a = make();
    const b = make();
    recycler.park(a);
    recycler.park(b);
    b.lost = true; // force-lost while pooled
    expect(recycler.acquire()).toBe(a); // b skipped
    expect(disposed).toEqual([b]); // and disposed
    expect(recycler.size()).toBe(0);
  });

  it('drop: removes a pooled canvas without disposing it', () => {
    const { recycler, disposed, make } = setup(4);
    const a = make();
    recycler.park(a);
    expect(recycler.drop(a)).toBe(true);
    expect(recycler.size()).toBe(0);
    expect(disposed).toEqual([]);
    expect(recycler.drop(a)).toBe(false); // already gone
  });

  it('flush: disposes and drops every pooled canvas (editor teardown)', () => {
    const { recycler, disposed, make } = setup(4);
    const a = make();
    const b = make();
    const c = make();
    recycler.park(a);
    recycler.park(b);
    recycler.park(c);
    recycler.flush();
    expect(recycler.size()).toBe(0);
    expect(new Set(disposed)).toEqual(new Set([a, b, c]));
    // flush 후 재-acquire는 miss.
    expect(recycler.acquire()).toBeUndefined();
  });

  it('cap 0: never pools', () => {
    const { recycler, make } = setup(0);
    expect(recycler.park(make())).toBe('lost');
    expect(recycler.size()).toBe(0);
  });
});
