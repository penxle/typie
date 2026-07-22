import crypto from 'node:crypto';
import { DocumentContentRating, EntityState, EntityVisibility } from '@typie/lib/enums';
import { asc, eq, inArray, sql } from 'drizzle-orm';
import { Hono } from 'hono';
import { z } from 'zod';
import { db, dbr, DocumentBundles, Prompts } from '#/db/index.ts';
import { env } from '#/env.ts';
import { wasmThread } from '#/utils/wasm-thread.ts';
import type { Env } from '#/context.ts';

export const internal = new Hono<Env>();

export const verifyInternalKey = (header: string | undefined, key: string): boolean => {
  if (!header) return false;
  const token = header.startsWith('Bearer ') ? header.slice(7) : header;
  const a = Buffer.from(token);
  const b = Buffer.from(key);
  if (a.length !== b.length) return false;
  return crypto.timingSafeEqual(a, b);
};

export const hangulRatio = (text: string): number => {
  const chars = [...text.replaceAll(/\s/g, '')];
  if (chars.length === 0) return 0;
  return chars.filter((ch) => /[가-힣ㄱ-ㅎㅏ-ㅣ]/.test(ch)).length / chars.length;
};

export const promptUpdateSchema = z.object({
  model: z.string().min(1),
  effort: z.string().nullable(),
  systemPrompt: z.string().min(1),
  toolDescriptions: z.record(z.string(), z.unknown()),
});

// cspell:disable-next-line
const PROMPT_IDS = ['PRMT0SUMMARIZE', 'PRMT0META', 'PRMT0ANALYZE'];

internal.use('*', async (c, next) => {
  if (!verifyInternalKey(c.req.header('authorization'), env.INTERNAL_API_KEY)) {
    return c.json({ error: 'unauthorized' }, 401);
  }
  await next();
});

internal.get('/prompts', async (c) => {
  const rows = await dbr
    .select({
      id: Prompts.id,
      model: Prompts.model,
      effort: Prompts.effort,
      systemPrompt: Prompts.systemPrompt,
      toolDescriptions: Prompts.toolDescriptions,
    })
    .from(Prompts)
    .where(inArray(Prompts.id, PROMPT_IDS));

  return c.json({ prompts: rows });
});

internal.put('/prompts/:id', async (c) => {
  const id = c.req.param('id');
  if (!PROMPT_IDS.includes(id)) {
    return c.json({ error: 'not found' }, 404);
  }

  const parsed = promptUpdateSchema.safeParse(await c.req.json());
  if (!parsed.success) {
    return c.json({ error: 'invalid payload' }, 400);
  }

  const p = parsed.data;
  await db
    .update(Prompts)
    .set({ model: p.model, effort: p.effort, systemPrompt: p.systemPrompt, toolDescriptions: p.toolDescriptions, updatedAt: sql`now()` })
    .where(eq(Prompts.id, id));

  return c.json({ ok: true });
});

const candidatesSchema = z.object({
  limit: z.number().int().min(1).max(400).default(400),
  minLength: z.number().int().default(3000),
  maxLength: z.number().int().default(30_000),
});

internal.post('/corpus/candidates', async (c) => {
  const parsed = candidatesSchema.safeParse(await c.req.json().catch(() => ({})));
  if (!parsed.success) {
    return c.json({ error: 'invalid payload' }, 400);
  }

  const { limit, minLength, maxLength } = parsed.data;
  const rows = await dbr.execute<{ document_id: string; text: string; character_count: number }>(sql`
    select dc.document_id, dc.text, dc.character_count
    from document_contents dc tablesample system (10)
    join documents d on d.id = dc.document_id
    join entities e on e.id = d.entity_id
    where dc.character_count between ${minLength} and ${maxLength}
      and e.visibility = ${EntityVisibility.PUBLIC} and e.state = ${EntityState.ACTIVE}
      and d.password is null and d.content_rating = ${DocumentContentRating.ALL}
    limit ${limit}
  `);

  const seen = new Set<string>();
  const candidates: { documentId: string; text: string; characterCount: number }[] = [];
  for (const row of rows) {
    if (hangulRatio(row.text) < 0.7) continue;

    const hash = crypto.createHash('sha256').update(row.text).digest('hex');
    if (seen.has(hash)) continue;
    seen.add(hash);

    candidates.push({ documentId: row.document_id, text: row.text, characterCount: row.character_count });
  }

  return c.json({ candidates });
});

const extractSchema = z.object({ documentIds: z.array(z.string().min(1)).min(1).max(5) });

internal.post('/corpus/extract', async (c) => {
  const parsed = extractSchema.safeParse(await c.req.json());
  if (!parsed.success) {
    return c.json({ error: 'invalid payload' }, 400);
  }

  const results: { documentId: string; prose: string | null }[] = [];
  for (const documentId of parsed.data.documentIds) {
    try {
      const bundles = await dbr
        .select({ payload: DocumentBundles.payload })
        .from(DocumentBundles)
        .where(eq(DocumentBundles.documentId, documentId))
        .orderBy(asc(DocumentBundles.seq));

      const total = bundles.reduce((n, row) => n + row.payload.length, 0);
      if (total === 0) {
        results.push({ documentId, prose: null });
        continue;
      }

      const graph = new Uint8Array(total);
      let offset = 0;
      for (const row of bundles) {
        graph.set(row.payload, offset);
        offset += row.payload.length;
      }

      const { result } = await wasmThread.extractProse(graph);
      results.push({ documentId, prose: result });
    } catch (err) {
      console.error(String(err));
      results.push({ documentId, prose: null });
    }
  }

  return c.json({ results });
});
