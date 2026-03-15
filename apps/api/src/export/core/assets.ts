import { GetObjectCommand } from '@aws-sdk/client-s3';
import * as Sentry from '@sentry/node';
import { inArray } from 'drizzle-orm';
import { db, Embeds, Images } from '#/db/index.ts';
import * as aws from '#/external/aws.ts';
import type { EmbedInfo, ImageAsset } from './types.ts';

export function collectNodeIds(nodes: Record<string, { type: string; id?: string }>, type: string): string[] {
  const ids: string[] = [];
  for (const entry of Object.values(nodes)) {
    if (entry.type === type && entry.id) {
      ids.push(entry.id);
    }
  }
  return ids;
}

export async function loadEmbeds(ids: string[]): Promise<Map<string, EmbedInfo>> {
  if (ids.length === 0) return new Map();
  const rows = await db.select({ id: Embeds.id, url: Embeds.url, title: Embeds.title }).from(Embeds).where(inArray(Embeds.id, ids));
  return new Map(rows.map((r) => [r.id, { url: r.url, title: r.title }]));
}

export async function loadImageAssets(ids: readonly string[]): Promise<Map<string, ImageAsset>> {
  if (ids.length === 0) return new Map();

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

export function mapFormat(format: string): string {
  if (format === 'image/jpeg' || format === 'image/jpg') return 'jpg';
  if (format === 'image/gif') return 'gif';
  if (format === 'image/bmp') return 'bmp';
  return 'png';
}
