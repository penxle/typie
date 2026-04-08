import { createHash } from 'node:crypto';
import { GetObjectCommand } from '@aws-sdk/client-s3';
import defaultFontFamilies from '@typie/editor/font/defaults.json' with { type: 'json' };
import { eq } from 'drizzle-orm';
import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { HTTPException } from 'hono/http-exception';
import { redis } from '#/cache.ts';
import { db, first, Fonts } from '#/db/index.ts';
import * as aws from '#/external/aws.ts';
import { normalizeHexColor } from '#/utils/color.ts';
import { decompressZstd } from '#/utils/compression.ts';
import { outlineTextToSvg } from '#/utils/font.ts';
import { normalizeSpecimenFallbacks, renderFontSpecimenSvg } from './font-specimen.ts';
import type { Env } from '#/context.ts';

const defaultFontMap = new Map(defaultFontFamilies.flatMap((f) => f.fonts.map((v) => [v.id, v.path] as const)));

export const font = new Hono<Env>();

font.use(cors());

font.get('/:fontId/specimen', async (c) => {
  const fontId = c.req.param('fontId');
  const url = new URL(c.req.url);
  const text = url.searchParams.get('text')?.trim();
  const fallbacks = normalizeSpecimenFallbacks(text ?? '', url.searchParams.getAll('fallbacks'));
  const rawColor = url.searchParams.get('color');
  const color = normalizeHexColor(rawColor);

  if (!text) {
    throw new HTTPException(400);
  }

  if (rawColor != null && color == null) {
    throw new HTTPException(400);
  }

  const textHash = createHash('sha256').update(JSON.stringify({ text, fallbacks, color })).digest('hex').slice(0, 16);
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

  const svg = await renderFontSpecimenSvg({
    text,
    fallbacks,
    color,
    renderTextToSvg: (candidate) => outlineTextToSvg(fontData, candidate),
  });

  await redis.set(cacheKey, svg, 'EX', 60 * 60 * 24 * 30);

  return c.body(svg, {
    headers: { 'Content-Type': 'image/svg+xml' },
  });
});
