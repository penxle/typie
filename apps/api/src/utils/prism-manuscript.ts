import { createHash } from 'node:crypto';
import { TypieError } from '@typie/lib/errors';
import { eq } from 'drizzle-orm';
import { redis } from '#/cache.ts';
import { dbr, Documents, first } from '#/db/index.ts';
import { getLiveHeads, readMergedGraph } from './changeset.ts';
import { wasmThread } from './wasm-thread.ts';
import type { Snapshot } from './prism-review-core.ts';

export type Manuscript = Snapshot & { characterCount: number; heads: Uint8Array };

type Extracted = { content: string; characterCount: number; heads: Uint8Array };
type CachedExtracted = { content: string; characterCount: number; heads: string };

const CACHE_TTL_SECONDS = 60 * 60;

// 페이로드가 heads를 싣게 되면서 키에 판을 매겼다 — heads 없는 옛 항목은 그냥 빗나가게 둔다
const cacheKey = (documentId: string, heads: Uint8Array): string =>
  `prism:manuscript:v2:${documentId}:${createHash('sha256').update(heads).digest('hex').slice(0, 16)}`;

const extractManuscript = async (documentId: string): Promise<Extracted> => {
  const graph = await readMergedGraph(documentId);
  if (graph.length === 0) throw new TypieError({ code: 'prism_manuscript_empty', status: 400 });

  const extracted = await wasmThread.extractProse(graph).catch(() => {
    throw new TypieError({ code: 'prism_extract_failed', status: 502 });
  });

  const content = extracted.result.text;
  if (content === null || content.trim().length === 0) throw new TypieError({ code: 'prism_manuscript_empty', status: 400 });

  return { content, characterCount: extracted.result.characterCount, heads: extracted.result.heads };
};

export const snapshotManuscript = async (documentId: string): Promise<Manuscript> => {
  const head = await dbr
    .select({ title: Documents.title, subtitle: Documents.subtitle })
    .from(Documents)
    .where(eq(Documents.id, documentId))
    .then(first);
  if (!head) throw new TypieError({ code: 'not_found', status: 404 });

  const heads = await getLiveHeads(documentId);
  const key = heads === null ? null : cacheKey(documentId, heads);
  const cached = key === null ? null : await redis.get(key);
  if (cached !== null) {
    const parsed = JSON.parse(cached) as CachedExtracted;
    return {
      title: head.title,
      subtitle: head.subtitle,
      content: parsed.content,
      characterCount: parsed.characterCount,
      heads: Uint8Array.fromBase64(parsed.heads),
    };
  }

  const extracted = await extractManuscript(documentId);
  if (key !== null)
    await redis.set(
      key,
      JSON.stringify({ ...extracted, heads: extracted.heads.toBase64() } satisfies CachedExtracted),
      'EX',
      CACHE_TTL_SECONDS,
    );

  return { title: head.title, subtitle: head.subtitle, ...extracted };
};
