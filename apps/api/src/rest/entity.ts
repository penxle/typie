import { createHmac } from 'node:crypto';
import { EntityState, EntityType } from '@typie/lib/enums';
import { and, eq } from 'drizzle-orm';
import { Hono } from 'hono';
import { HTTPException } from 'hono/http-exception';
import { match } from 'ts-pattern';
import { redis } from '#/cache.ts';
import { db, DocumentContents, Documents, Entities, first } from '#/db/index.ts';
import { env } from '#/env.ts';
import { generateDocumentPreview } from '#/export/preview/index.ts';
import { buildExportFonts } from '#/graphql/resolvers/export.ts';
import type { Context } from 'hono';
import type { Env } from '#/context.ts';
import type { PreviewTheme } from '#/export/preview/index.ts';

export const entity = new Hono<Env>();

const CACHE_TTL = 60 * 60 * 24 * 7; // 1주일
const DEFAULT_WIDTH = 400;
const MAX_WIDTH = 1200;

function verifySignature(entityId: string, expires: string, sig: string): boolean {
  const now = Math.floor(Date.now() / 1000);
  const expiresNum = Number.parseInt(expires, 10);

  if (!Number.isFinite(expiresNum) || expiresNum < now) {
    return false;
  }

  const expected = createHmac('sha256', env.PREVIEW_SIGNING_SECRET).update(`${entityId}:${expires}`).digest('hex').slice(0, 16);

  return sig === expected;
}

entity.get('/:entityId/preview', async (c) => {
  const entityId = c.req.param('entityId');
  const expires = c.req.query('expires');
  const sig = c.req.query('sig');
  const widthParam = c.req.query('w');

  if (!expires || !sig) {
    throw new HTTPException(401);
  }

  if (!verifySignature(entityId, expires, sig)) {
    throw new HTTPException(401);
  }

  const width = Math.min(Math.max(Number.parseInt(widthParam ?? '', 10) || DEFAULT_WIDTH, 1), MAX_WIDTH);
  const theme = c.req.query('theme') === 'dark' ? ('dark' as const) : ('light' as const);

  const entity = await db
    .select({ type: Entities.type, userId: Entities.userId })
    .from(Entities)
    .where(and(eq(Entities.id, entityId), eq(Entities.state, EntityState.ACTIVE)))
    .then(first);

  if (!entity) {
    throw new HTTPException(404);
  }

  return match(entity.type)
    .with(EntityType.DOCUMENT, () => renderDocumentPreview(c, entityId, entity.userId, width, theme, expires))
    .otherwise(() => {
      throw new HTTPException(422);
    });
});

async function renderDocumentPreview(
  c: Context<Env>,
  entityId: string,
  userId: string,
  width: number,
  theme: PreviewTheme,
  expires: string,
) {
  const document = await db
    .select({
      id: Documents.id,
      title: Documents.title,
      subtitle: Documents.subtitle,
    })
    .from(Documents)
    .where(eq(Documents.entityId, entityId))
    .then(first);

  if (!document) {
    throw new HTTPException(404);
  }

  // Redis 캐시 확인
  const cacheKey = `document:preview:${document.id}:${theme}:${width}`;
  const cached = await redis.getBuffer(cacheKey);
  if (cached) {
    const maxAge = Number.parseInt(expires, 10) - Math.floor(Date.now() / 1000);
    return c.body(cached as unknown as string, {
      headers: {
        'Content-Type': 'image/webp',
        'Cache-Control': `public, max-age=${Math.max(maxAge, 0)}`,
      },
    });
  }

  const content = await db
    .select({ snapshot: DocumentContents.snapshot })
    .from(DocumentContents)
    .where(eq(DocumentContents.documentId, document.id))
    .then(first);

  if (!content) {
    throw new HTTPException(404);
  }

  // 폰트 빌드
  const exportFonts = await buildExportFonts(userId);
  const fonts = exportFonts.map((f) => ({
    familyName: f.family,
    fonts: f.weights.map((w) => ({ weight: w.weight, url: w.url })),
  }));

  try {
    const webp = await generateDocumentPreview({
      snapshot: content.snapshot,
      title: document.title || '(제목 없음)',
      subtitle: document.subtitle,
      fonts,
      width,
      theme,
    });

    // Redis 저장
    await redis.set(cacheKey, Buffer.from(webp), 'EX', CACHE_TTL);

    const maxAge = Number.parseInt(expires, 10) - Math.floor(Date.now() / 1000);
    return c.body(webp as unknown as string, {
      headers: {
        'Content-Type': 'image/webp',
        'Cache-Control': `public, max-age=${Math.max(maxAge, 0)}`,
      },
    });
  } catch (err) {
    if (err instanceof Error && err.message === 'Empty document') {
      throw new HTTPException(422);
    }
    throw err;
  }
}
