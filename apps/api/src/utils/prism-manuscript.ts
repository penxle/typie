import { createHash } from 'node:crypto';
import { TypieError } from '@typie/lib/errors';
import { eq } from 'drizzle-orm';
import { redis } from '#/cache.ts';
import { dbr, Documents, first } from '#/db/index.ts';
import { getLiveHeads, readMergedGraph } from './changeset.ts';
import { wasmThread } from './wasm-thread.ts';
import type { Snapshot } from './prism-review-core.ts';

export type Manuscript = Snapshot & { characterCount: number };

type Extracted = { content: string; characterCount: number };

const CACHE_TTL_SECONDS = 60 * 60;

const cacheKey = (documentId: string, heads: Uint8Array): string =>
  `prism:manuscript:${documentId}:${createHash('sha256').update(heads).digest('hex').slice(0, 16)}`;

const extractManuscript = async (documentId: string): Promise<Extracted> => {
  const graph = await readMergedGraph(documentId);
  if (graph.length === 0) throw new TypieError({ code: 'prism_manuscript_empty', status: 400 });

  const extracted = await wasmThread.extractProse(graph).catch(() => {
    throw new TypieError({ code: 'prism_extract_failed', status: 502 });
  });

  const content = extracted.result.text;
  if (content === null || content.trim().length === 0) throw new TypieError({ code: 'prism_manuscript_empty', status: 400 });

  return { content, characterCount: extracted.result.characterCount };
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
  if (cached !== null) return { title: head.title, subtitle: head.subtitle, ...(JSON.parse(cached) as Extracted) };

  const extracted = await extractManuscript(documentId);
  if (key !== null) await redis.set(key, JSON.stringify(extracted), 'EX', CACHE_TTL_SECONDS);

  return { title: head.title, subtitle: head.subtitle, ...extracted };
};
