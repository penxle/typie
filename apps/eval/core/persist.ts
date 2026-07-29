import { eq, inArray } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { chunkRows, inChunks, ItemAnchors, ItemLinks, RunItems } from './db.ts';
import type { ItemDraft } from './contracts.ts';
import type { Db } from './db.ts';

export type LinkRow = { itemId: string; targetItemId: string; ord: number };

export const resolveLinks = (ids: string[], selfIndex: number, links: number[]): LinkRow[] => {
  const seen = new Set<number>();
  const rows: LinkRow[] = [];
  for (const index of links) {
    if (!Number.isSafeInteger(index) || index < 0 || index >= ids.length) continue;
    if (index === selfIndex || seen.has(index)) continue;
    seen.add(index);
    rows.push({ itemId: ids[selfIndex], targetItemId: ids[index], ord: rows.length });
  }
  return rows;
};

// 저장은 코어가 단독으로 소유한다. 세대마다 제각각이던 persist가 하나가 되면서 멱등성이
// 구조적으로 보장된다 — 구 분석 파이프라인은 이 삭제를 하지 않아 리플레이 시 지적이 중복됐다.
export const persistItems = async (db: Db, runId: string, items: ItemDraft[]): Promise<void> => {
  const stale = await db.select({ id: RunItems.id }).from(RunItems).where(eq(RunItems.runId, runId));
  if (stale.length > 0) {
    const staleIds = stale.map((r) => r.id);
    await inChunks(staleIds, (chunk) =>
      db.delete(ItemAnchors).where(inArray(ItemAnchors.itemId, chunk)).returning({ id: ItemAnchors.itemId }),
    );
    await inChunks(staleIds, (chunk) => db.delete(ItemLinks).where(inArray(ItemLinks.itemId, chunk)).returning({ id: ItemLinks.itemId }));
    await db.delete(RunItems).where(eq(RunItems.runId, runId));
  }

  if (items.length === 0) return;

  const ids = items.map(() => nanoid());

  const itemRows = items.map((item, i) => ({
    id: ids[i],
    runId,
    kind: item.kind,
    ord: item.ord,
    body: item.body,
    facets: item.facets,
  }));
  const anchorRows = items.flatMap((item, i) =>
    item.anchors.map((anchor, ord) => ({
      id: nanoid(),
      itemId: ids[i],
      ord,
      startText: anchor.quoteStart,
      endText: anchor.quoteEnd,
      matchStart: anchor.matchStart,
      matchEnd: anchor.matchEnd,
      note: anchor.note ?? null,
    })),
  );
  const linkRows = items.flatMap((item, i) => resolveLinks(ids, i, item.links));

  const statements: Promise<unknown>[] = [];
  chunkRows(itemRows, 6, (chunk) => {
    statements.push(db.insert(RunItems).values(chunk));
  });
  chunkRows(anchorRows, 8, (chunk) => {
    statements.push(db.insert(ItemAnchors).values(chunk));
  });
  chunkRows(linkRows, 3, (chunk) => {
    statements.push(db.insert(ItemLinks).values(chunk));
  });
  for (const statement of statements) await statement;
};
