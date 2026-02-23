import { GetObjectCommand } from '@aws-sdk/client-s3';
import { inArray } from 'drizzle-orm';
import { db, Images } from '@/db';
import * as aws from '@/external/aws';
import type { VectorPage } from './vector';

export type ResolvedExternalImage = {
  id: string;
  format: string;
  width: number;
  height: number;
  bytes: Uint8Array;
};

const IMAGE_BUCKET = 'typie-usercontents';

const isImageFormat = (format: string): boolean => format.startsWith('image/');

const collectImageIds = (pages: VectorPage[]): string[] => {
  const ids = new Set<string>();

  for (const page of pages) {
    for (const external of page.externalElements) {
      if (external.data.type !== 'image') continue;
      if (!external.data.id) continue;
      ids.add(external.data.id);
    }
  }

  return [...ids];
};

export async function resolveExternalImages(pages: VectorPage[]): Promise<Map<string, ResolvedExternalImage>> {
  const imageIds = collectImageIds(pages);
  if (imageIds.length === 0) {
    return new Map();
  }

  const images = await db
    .select({
      id: Images.id,
      format: Images.format,
      width: Images.width,
      height: Images.height,
      path: Images.path,
    })
    .from(Images)
    .where(inArray(Images.id, imageIds));

  const resolved = new Map<string, ResolvedExternalImage>();

  await Promise.all(
    images.map(async (image) => {
      if (!isImageFormat(image.format)) {
        return;
      }

      try {
        const object = await aws.s3.send(
          new GetObjectCommand({
            Bucket: IMAGE_BUCKET,
            Key: `images/${image.path}`,
          }),
        );

        if (!object.Body) {
          return;
        }

        const bytes = await object.Body.transformToByteArray();
        resolved.set(image.id, {
          id: image.id,
          format: image.format,
          width: image.width,
          height: image.height,
          bytes,
        });
      } catch (err) {
        console.warn(`[pdf] failed to resolve image ${image.id}`, err);
      }
    }),
  );

  return resolved;
}
