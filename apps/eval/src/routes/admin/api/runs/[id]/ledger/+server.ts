import { error, json } from '@sveltejs/kit';
import { and, gte, inArray, lt } from 'drizzle-orm';
import { createDb, StageCache } from '$lib/server/db/index.ts';
import type { RequestHandler } from './$types';

// 문서 하나의 스테이지별 도구 원장. 워크플로가 stage_cache에 남긴 ledger/*를 그대로 노출한다 —
// 진단용이므로 가공하지 않고, 없는 스테이지는 결과에서 빠진다(구 파이프라인 실행은 빈 목록).
type ToolRecord =
  | { turn: number; tool: 'read'; start: number; end: number }
  | { turn: number; tool: 'grep'; pattern: string; total: number }
  | { turn: number; tool: 'search'; query: string; hits: number };
type LedgerEvent = { turn?: number; kind: string; detail: string };
type StageLedger = { stage: string; tools: ToolRecord[]; events: LedgerEvent[]; live: boolean };

const STAGE_ORDER = ['research', 'plan-draft', 'plan-revise-0', 'plan-revise-1', 'plan-revise-2', 'plan', 'execute', 'local', 'compose'];

// 통합 원장(ledger/*)은 스테이지가 끝나야 영속된다. 진행 중 스테이지는 턴 도구 캐시에서
// 재구성한다 — 계획의 초안·수정 라운드가 분리돼 보이는 것은 진단에 오히려 유리하다.
const familyOf = (stage: string): string => (stage === 'plan-draft' || stage.startsWith('plan-revise') ? 'plan' : stage);

export const GET: RequestHandler = async ({ params, url, platform }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }
  const documentId = url.searchParams.get('documentId');
  if (!documentId) {
    error(400, 'documentId required');
  }

  const db = createDb(platform.env.DB);
  const docPrefix = `analysis/${params.id}/${documentId}/`;
  // D1은 긴 LIKE 패턴을 거부한다(SQLITE_ERROR: pattern too complex — runId+documentId 접두부가
  // 한계를 넘는다). 접두부 스캔은 범위 비교로 하고, 턴 캐시 본문이 크므로 키만 먼저 훑는다.
  const keys = await db
    .select({ key: StageCache.key })
    .from(StageCache)
    .where(and(gte(StageCache.key, docPrefix), lt(StageCache.key, docPrefix + '\u{FFFF}')));

  const ledgerKeys: string[] = [];
  const toolsKeysByStage = new Map<string, string[]>();
  for (const { key } of keys) {
    const suffix = key.slice(docPrefix.length);
    if (suffix.startsWith('ledger/')) {
      ledgerKeys.push(key);
      continue;
    }
    const parts = suffix.split('/');
    if (parts.length === 3 && parts[1] === 'tools') {
      const list = toolsKeysByStage.get(parts[0]) ?? [];
      list.push(key);
      toolsKeysByStage.set(parts[0], list);
    }
  }

  const fetchValues = async (wanted: string[]) => {
    const out = new Map<string, unknown>();
    for (let i = 0; i < wanted.length; i += 80) {
      const chunk = await db
        .select()
        .from(StageCache)
        .where(inArray(StageCache.key, wanted.slice(i, i + 80)));
      for (const row of chunk) out.set(row.key, row.value);
    }
    return out;
  };

  const ledgerValues = await fetchValues(ledgerKeys);
  const persisted: StageLedger[] = [...ledgerValues.entries()].map(([key, raw]) => {
    const stage = key.slice(docPrefix.length + 'ledger/'.length);
    const value = raw as LedgerEvent[] | { tools?: ToolRecord[]; events?: LedgerEvent[] };
    // compose 원장은 이벤트 배열만 저장된다 — 형태를 통일해 내보낸다.
    return Array.isArray(value)
      ? { stage, tools: [], events: value, live: false }
      : { stage, tools: value.tools ?? [], events: value.events ?? [], live: false };
  });

  // 영속 원장이 아직 없는 스테이지(진행 중)를 턴 도구 캐시에서 재구성한다. 이벤트(반려 등)는
  // 스테이지 완료 시에만 영속되므로 라이브 뷰에는 도구 호출만 보인다.
  const persistedFamilies = new Set(persisted.map((s) => s.stage));
  const liveStageNames = [...toolsKeysByStage.keys()].filter((stage) => !persistedFamilies.has(familyOf(stage)));
  const liveToolValues = await fetchValues(liveStageNames.flatMap((stage) => toolsKeysByStage.get(stage) ?? []));
  const live: StageLedger[] = liveStageNames.map((stage) => {
    const tools = (toolsKeysByStage.get(stage) ?? [])
      .map((key) => {
        const turn = Number(key.split('/').at(-1));
        const value = liveToolValues.get(key) as { value?: { records?: ToolRecord[] } } | undefined;
        return { turn, records: value?.value?.records ?? [] };
      })
      .toSorted((a, b) => a.turn - b.turn)
      .flatMap((t) => t.records);
    return { stage, tools, events: [], live: true };
  });

  const stages: StageLedger[] = [...persisted, ...live].toSorted((a, b) => {
    const ai = STAGE_ORDER.indexOf(a.stage);
    const bi = STAGE_ORDER.indexOf(b.stage);
    return (ai === -1 ? STAGE_ORDER.length : ai) - (bi === -1 ? STAGE_ORDER.length : bi) || a.stage.localeCompare(b.stage);
  });

  return json({ stages });
};
