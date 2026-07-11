type Asset = { id: string };

type AssetHydratorOptions<T extends Asset> = {
  hasAsset: (id: string) => boolean;
  fetchAssets: (ids: string[]) => Promise<readonly T[]>;
  putAsset: (asset: T) => void;
  wait?: (delay: number) => Promise<void>;
  retryDelays?: number[];
  batchSize?: number;
};

export const createAssetHydrator = <T extends Asset>({
  hasAsset,
  fetchAssets,
  putAsset,
  wait = (delay) => new Promise((resolve) => setTimeout(resolve, delay)),
  retryDelays = [0, 500, 1500],
  batchSize = 50,
}: AssetHydratorOptions<T>) => {
  let referencedIds = new Set<string>();
  const suppressedIds = new Set<string>();
  let queued = false;
  let destroyed = false;
  let running: Promise<void> | null = null;

  const hydrate = async () => {
    while (!destroyed) {
      const batch = [...referencedIds].filter((id) => !hasAsset(id) && !suppressedIds.has(id)).slice(0, batchSize);
      if (batch.length === 0) return;

      let missing = batch;
      for (const delay of retryDelays) {
        if (delay > 0) await wait(delay);
        if (destroyed) return;

        missing = missing.filter((id) => referencedIds.has(id) && !hasAsset(id));
        if (missing.length === 0) break;

        let fetched: readonly T[];
        try {
          fetched = await fetchAssets(missing);
        } catch {
          for (const id of missing) {
            if (referencedIds.has(id) && !hasAsset(id)) suppressedIds.add(id);
          }
          return;
        }
        if (destroyed) return;

        const requested = new Set(missing);
        for (const asset of fetched) {
          if (!(requested.has(asset.id) && referencedIds.has(asset.id))) {
            continue;
          }

          putAsset(asset);
          suppressedIds.delete(asset.id);
        }
      }

      for (const id of missing) {
        if (referencedIds.has(id) && !hasAsset(id)) {
          suppressedIds.add(id);
        }
      }
    }
  };

  const schedule = () => {
    if (destroyed) return Promise.resolve();
    queued = true;
    running ??= (async () => {
      while (queued && !destroyed) {
        queued = false;
        await hydrate();
      }
    })().finally(() => {
      running = null;
    });
    return running;
  };

  return {
    update(ids: Iterable<string>) {
      const nextIds = new Set(ids);
      for (const id of referencedIds) {
        if (!nextIds.has(id)) suppressedIds.delete(id);
      }
      referencedIds = nextIds;
      return schedule();
    },
    retry() {
      suppressedIds.clear();
      return schedule();
    },
    destroy() {
      destroyed = true;
      referencedIds.clear();
      suppressedIds.clear();
    },
  };
};
