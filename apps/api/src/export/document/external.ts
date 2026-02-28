import { GetObjectCommand } from '@aws-sdk/client-s3';
import * as Sentry from '@sentry/bun';
import DataLoader from 'dataloader';
import { inArray } from 'drizzle-orm';
import { db, decodeDbId, Images, TableCode } from '@/db';
import * as aws from '@/external/aws';
import type { ExternalElement } from './slate';

export type ImageAsset = {
  type: 'image';
  id: string;
  format: string;
  width: number;
  height: number;
  bytes: Uint8Array;
};

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

async function loadImageAssets(ids: readonly string[]): Promise<Map<string, ImageAsset>> {
  const images = await db
    .select({
      id: Images.id,
      format: Images.format,
      width: Images.width,
      height: Images.height,
      path: Images.path,
    })
    .from(Images)
    .where(inArray(Images.id, [...ids]));

  const assets = new Map<string, ImageAsset>();

  await Promise.all(
    images.map(async (image) => {
      try {
        const object = await aws.s3.send(
          new GetObjectCommand({
            Bucket: 'typie-usercontents',
            Key: `images/${image.path}`,
          }),
        );

        if (!object.Body) {
          return;
        }

        const bytes = await object.Body.transformToByteArray();

        assets.set(image.id, {
          type: 'image',
          id: image.id,
          format: image.format,
          width: image.width,
          height: image.height,
          bytes,
        });
      } catch (err) {
        Sentry.captureException(err);
      }
    }),
  );

  return assets;
}

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
