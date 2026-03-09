import { createHash } from 'node:crypto';
import { GetObjectCommand } from '@aws-sdk/client-s3';
import defaultFontFamilies from '@typie/editor/font/defaults.json' with { type: 'json' };
import { eq } from 'drizzle-orm';
import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { HTTPException } from 'hono/http-exception';
import { redis } from '@/cache';
import { db, first, Fonts } from '@/db';
import * as aws from '@/external/aws';
import { decompressZstd } from '@/utils/compression';
import { outlineTextToSvg } from '@/utils/font';
import type { Env } from '@/context';

const defaultFontMap = new Map(defaultFontFamilies.flatMap((f) => f.fonts.map((v) => [v.id, v.path] as const)));

export const font = new Hono<Env>();

font.use(cors());

font.get('/:fontId/specimen', async (c) => {
  const fontId = c.req.param('fontId');
  const text = c.req.query('text');

  if (!text) {
    throw new HTTPException(400);
  }

  const textHash = createHash('sha256').update(text).digest('hex').slice(0, 16);
  const cacheKey = `font:specimen:${fontId}:${textHash}`;
  const cached = await redis.get(cacheKey);

  if (cached) {
    return c.body(cached, {
      headers: { 'Content-Type': 'image/svg+xml' },
    });
  }

  const defaultPath = defaultFontMap.get(fontId);

  let bucket: string;
  let key: string;

  if (defaultPath) {
    bucket = 'typie-cdn';
    key = `editor/fonts/${defaultPath}/original.bin`;
  } else {
    const row = await db.select({ path: Fonts.path }).from(Fonts).where(eq(Fonts.id, fontId)).then(first);

    if (!row) {
      throw new HTTPException(404);
    }

    bucket = 'typie-usercontents';
    key = `fonts/${row.path}/original.bin`;
  }

  const object = await aws.s3.send(new GetObjectCommand({ Bucket: bucket, Key: key }));

  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  const compressed = await object.Body!.transformToByteArray();
  const fontData = await decompressZstd(compressed);

  let svg: string;
  try {
    svg = await outlineTextToSvg(fontData, text);
  } catch (err) {
    if (String(err).includes('missing glyph')) {
      throw new HTTPException(422);
    }
    throw err;
  }

  await redis.set(cacheKey, svg, 'EX', 60 * 60 * 24 * 30);

  return c.body(svg, {
    headers: { 'Content-Type': 'image/svg+xml' },
  });
});
