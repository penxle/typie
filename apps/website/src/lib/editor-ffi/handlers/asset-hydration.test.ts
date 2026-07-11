import { describe, expect, it, vi } from 'vitest';
import { createAssetHydrator } from './asset-hydration';

type Asset = { id: string };

const createHarness = (fetchAssets: (ids: string[]) => Promise<Asset[]>) => {
  const assets = new Map<string, Asset>();
  const wait = vi.fn(() => Promise.resolve());
  const hydrator = createAssetHydrator({
    hasAsset: (id) => assets.has(id),
    fetchAssets,
    putAsset: (asset) => assets.set(asset.id, asset),
    wait,
    retryDelays: [0, 100, 300],
  });
  return { assets, hydrator, wait };
};

describe('asset hydration', () => {
  it('fetches only referenced assets missing from the local maps', async () => {
    const fetchAssets = vi.fn(async (ids: string[]) => ids.map((id) => ({ id })));
    const { assets, hydrator } = createHarness(fetchAssets);
    assets.set('present', { id: 'present' });

    await hydrator.update(['present', 'missing', 'missing']);

    expect(fetchAssets).toHaveBeenCalledWith(['missing']);
    expect(assets.has('missing')).toBe(true);
  });

  it('retries IDs that are not materialized yet', async () => {
    let attempt = 0;
    const fetchAssets = vi.fn(async () => (++attempt < 3 ? [] : [{ id: 'late' }]));
    const { assets, hydrator, wait } = createHarness(fetchAssets);

    await hydrator.update(['late']);

    expect(fetchAssets).toHaveBeenCalledTimes(3);
    expect(wait).toHaveBeenNthCalledWith(1, 100);
    expect(wait).toHaveBeenNthCalledWith(2, 300);
    expect(assets.has('late')).toBe(true);
  });

  it('splits large reference sets into bounded batches', async () => {
    const fetchAssets = vi.fn(async (ids: string[]) => ids.map((id) => ({ id })));
    const { hydrator } = createHarness(fetchAssets);
    const ids = Array.from({ length: 51 }, (_, index) => `asset-${index}`);

    await hydrator.update(ids);

    expect(fetchAssets.mock.calls.map(([batch]) => batch.length)).toEqual([50, 1]);
  });

  it('waits for an explicit retry after a network failure', async () => {
    let online = false;
    const fetchAssets = vi.fn(async () => {
      if (!online) throw new Error('offline');
      return [{ id: 'asset-1' }];
    });
    const { assets, hydrator } = createHarness(fetchAssets);

    await hydrator.update(['asset-1']);
    await hydrator.update(['asset-1']);
    expect(fetchAssets).toHaveBeenCalledTimes(1);
    expect(assets.has('asset-1')).toBe(false);

    online = true;
    await hydrator.retry();

    expect(fetchAssets).toHaveBeenCalledTimes(2);
    expect(assets.has('asset-1')).toBe(true);
  });

  it('does not restart materialization retries until recovery or a new reference', async () => {
    const fetchAssets = vi.fn(async () => []);
    const { hydrator } = createHarness(fetchAssets);

    await hydrator.update(['missing']);
    await hydrator.update(['missing']);
    expect(fetchAssets).toHaveBeenCalledTimes(3);

    await hydrator.update([]);
    await hydrator.update(['missing']);
    expect(fetchAssets).toHaveBeenCalledTimes(6);
  });

  it('does not cache a result whose reference disappeared while fetching', async () => {
    const { promise, resolve } = Promise.withResolvers<Asset[]>();
    const { assets, hydrator } = createHarness(() => promise);

    const first = hydrator.update(['removed']);
    const second = hydrator.update([]);
    resolve([{ id: 'removed' }]);
    await Promise.all([first, second]);

    expect(assets.has('removed')).toBe(false);
  });

  it('does nothing after it is destroyed', async () => {
    const fetchAssets = vi.fn(async () => [{ id: 'asset-1' }]);
    const { hydrator } = createHarness(fetchAssets);
    hydrator.destroy();

    await hydrator.update(['asset-1']);

    expect(fetchAssets).not.toHaveBeenCalled();
  });
});
