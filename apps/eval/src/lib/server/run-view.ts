import { eq, inArray } from 'drizzle-orm';
import { Documents, inChunks, ItemAnchors, ItemLinks, Ledgers, PromptSets, RunItems, Runs } from '../../../core/db.ts';
import type { Db } from '../../../core/db.ts';

export type ViewAnchor = { startText: string; endText: string; matchStart: number | null; matchEnd: number | null; note: string | null };

export type ViewItem = {
  id: string;
  kind: string;
  ord: number;
  body: string;
  facets: Record<string, string>;
  anchors: ViewAnchor[];
  links: string[];
};

export type RunView = {
  runId: string;
  generationId: string | null;
  promptSetLabel: string | null;
  document: { id: string; refId: string; content: string; characterCount: number };
  items: ViewItem[];
};

type ItemRow = { id: string; kind: string; ord: number; body: string; facets: Record<string, string> };
type AnchorRow = { itemId: string; ord: number } & ViewAnchor;
type LinkRow = { itemId: string; targetItemId: string; ord: number };

export const assembleItems = (items: ItemRow[], anchors: AnchorRow[], links: LinkRow[]): ViewItem[] => {
  const anchorsOf = new Map<string, AnchorRow[]>();
  for (const anchor of anchors) {
    anchorsOf.set(anchor.itemId, [...(anchorsOf.get(anchor.itemId) ?? []), anchor]);
  }
  const linksOf = new Map<string, LinkRow[]>();
  for (const link of links) {
    linksOf.set(link.itemId, [...(linksOf.get(link.itemId) ?? []), link]);
  }

  return items.map((item) => ({
    id: item.id,
    kind: item.kind,
    ord: item.ord,
    body: item.body,
    facets: item.facets,
    anchors: (anchorsOf.get(item.id) ?? [])
      .toSorted((a, b) => a.ord - b.ord)
      .map(({ startText, endText, matchStart, matchEnd, note }) => ({ startText, endText, matchStart, matchEnd, note })),
    links: (linksOf.get(item.id) ?? []).toSorted((a, b) => a.ord - b.ord).map((l) => l.targetItemId),
  }));
};

// 세대 무관 원자료. 세대 렌더러는 이 형태만 받는다 — 코어는 facets 안을 들여다보지 않는다.
export const loadRunView = async (db: Db, runId: string): Promise<RunView | null> => {
  const [run] = await db
    .select({ id: Runs.id, documentId: Runs.documentId, generationId: PromptSets.generationId, promptSetLabel: PromptSets.label })
    .from(Runs)
    .leftJoin(PromptSets, eq(PromptSets.id, Runs.promptSetId))
    .where(eq(Runs.id, runId))
    .limit(1);
  if (!run) return null;

  const [document] = await db
    .select({ id: Documents.id, refId: Documents.refId, content: Documents.content, characterCount: Documents.characterCount })
    .from(Documents)
    .where(eq(Documents.id, run.documentId))
    .limit(1);
  if (!document) return null;

  const rows = await db
    .select({ id: RunItems.id, kind: RunItems.kind, ord: RunItems.ord, body: RunItems.body, facets: RunItems.facets })
    .from(RunItems)
    .where(eq(RunItems.runId, runId));
  const ids = rows.map((r) => r.id);

  const anchors = await inChunks(ids, (chunk) =>
    db
      .select({
        itemId: ItemAnchors.itemId,
        ord: ItemAnchors.ord,
        startText: ItemAnchors.startText,
        endText: ItemAnchors.endText,
        matchStart: ItemAnchors.matchStart,
        matchEnd: ItemAnchors.matchEnd,
        note: ItemAnchors.note,
      })
      .from(ItemAnchors)
      .where(inArray(ItemAnchors.itemId, chunk)),
  );
  const links = await inChunks(ids, (chunk) => db.select().from(ItemLinks).where(inArray(ItemLinks.itemId, chunk)));

  return {
    runId: run.id,
    generationId: run.generationId,
    promptSetLabel: run.promptSetLabel,
    document,
    items: assembleItems(rows, anchors, links),
  };
};

// 세대 전용 모달이 읽는 진단 기록. 코어는 값의 모양을 모르고 그대로 넘긴다.
export const loadLedgers = async (db: Db, runId: string, keys: string[]): Promise<Record<string, unknown>> => {
  if (keys.length === 0) return {};
  const rows = await db.select({ key: Ledgers.key, value: Ledgers.value }).from(Ledgers).where(eq(Ledgers.runId, runId));
  return Object.fromEntries(rows.filter((r) => keys.includes(r.key)).map((r) => [r.key, r.value]));
};
