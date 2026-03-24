import DataLoader from 'dataloader';
import { decodeDbId, TableCode } from '#/db/index.ts';
import { loadImageAssets } from './assets.ts';
import type { ExternalElement } from './slate.ts';
import type { ImageAsset } from './types.ts';

export type Asset = ImageAsset;

const FALLBACK_HEIGHT = 48;

export const computeDesiredSize = (external: ExternalElement, asset: Asset | undefined): { width: number; height: number } => {
  switch (external.data.type) {
    case 'image': {
      if (!asset || asset.type !== 'image' || asset.width <= 0 || asset.height <= 0) {
        return { width: external.bounds.width, height: FALLBACK_HEIGHT };
      }

      const widthLimit = external.bounds.width * external.data.proportion;
      const width = Math.min(asset.width, widthLimit);
      const height = width * (asset.height / asset.width);

      if (!Number.isFinite(height) || height <= 0) {
        return { width: external.bounds.width, height: FALLBACK_HEIGHT };
      }

      return { width, height };
    }
    default: {
      return { width: external.bounds.width, height: Math.max(external.bounds.height, FALLBACK_HEIGHT) };
    }
  }
};

function createAssetLoader(): DataLoader<string, Asset | null> {
  return new DataLoader<string, Asset | null>(async (ids) => {
    const imageIds = ids.filter((id) => decodeDbId(id) === TableCode.IMAGES);
    const imageAssets = imageIds.length > 0 ? await loadImageAssets(imageIds) : new Map<string, ImageAsset>();

    return ids.map((id) => imageAssets.get(id) ?? null);
  });
}

export async function resolveAssets(externals: readonly ExternalElement[]): Promise<Map<string, Asset>> {
  const loader = createAssetLoader();
  const assets = new Map<string, Asset>();

  await Promise.all(
    externals.map(async (ext) => {
      if (!ext.data.id) return;
      const asset = await loader.load(ext.data.id);
      if (asset) assets.set(ext.nodeId, asset);
    }),
  );

  return assets;
}
