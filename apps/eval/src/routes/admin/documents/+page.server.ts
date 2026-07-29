import { error, fail, redirect } from '@sveltejs/kit';
import { count, desc, eq, inArray, sql } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { createInternalApi } from '$lib/server/internal-api.ts';
import { spawnRuns } from '$lib/server/run-service.ts';
import { recentSamplings, refreshSampling, spawnSampling } from '$lib/server/sampling.ts';
import { createDb, Documents, inChunks, PromptSets, Rounds, Runs, Tasks } from '../../../../core/db.ts';
import { generationById } from '../../../../core/registry.ts';
import { GENRES } from '../../../../flows/src/genres.ts';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ platform }) => {
  if (!platform) error(500, 'platform unavailable');
  const db = createDb(platform.env.DB);

  // 죽은 표집이 'running'으로 남으면 새 표집 가드가 영구히 잠긴다 — 폴링이 이 로드를 지나므로
  // 여기서 인스턴스 상태를 맞추되, 응답에 기여하지 않는 대사이므로 반환 뒤 백그라운드로 미룬다.
  const samplings = await recentSamplings(db);
  const activeSamplings = samplings.filter((s) => s.status === 'running' || s.status === 'pending');
  const env = platform.env;
  platform.context?.waitUntil(
    Promise.all(activeSamplings.map((sampling) => refreshSampling(db, env, sampling.id))).catch((err) =>
      console.warn('표집 상태 대사 실패', err),
    ),
  );

  const documents = await db
    .select({
      id: Documents.id,
      refId: Documents.refId,
      kind: Documents.kind,
      characterCount: Documents.characterCount,
      // 개행 수는 따로 저장하지 않는다 — 본문을 통째로 끌어오지 않도록 셈만 SQL에 맡긴다.
      lineBreakCount: sql<number>`length(${Documents.content}) - length(replace(${Documents.content}, char(10), ''))`,
      createdAt: Documents.createdAt,
    })
    .from(Documents)
    .orderBy(desc(Documents.createdAt));

  const runs = await db.select({ documentId: Runs.documentId, status: Runs.status }).from(Runs);
  const runCount = new Map<string, number>();
  for (const run of runs) runCount.set(run.documentId, (runCount.get(run.documentId) ?? 0) + 1);

  // 한 번 라운드에 쓰인 문서는 다음 라운드에서 빠진다 — 어느 라운드에 갔는지 목록에서 바로 읽혀야
  // 후보에 안 뜨는 이유를 여기서 확인할 수 있다.
  const rounded = await db
    .select({ documentId: Runs.documentId, roundLabel: Rounds.label })
    .from(Tasks)
    .innerJoin(Runs, eq(Runs.id, Tasks.runId))
    .innerJoin(Rounds, eq(Rounds.id, Tasks.roundId));
  const roundsOf = new Map<string, string[]>();
  for (const row of rounded) {
    const labels = roundsOf.get(row.documentId) ?? [];
    if (!labels.includes(row.roundLabel)) labels.push(row.roundLabel);
    roundsOf.set(row.documentId, labels);
  }

  const sets = await db.select().from(PromptSets).orderBy(desc(PromptSets.createdAt));

  // 동결 분포 — 표집이 한쪽으로 쏠렸는지 읽는 자리다. 반입 문서는 장르를 받지 않으므로 함께 세면
  // 「장르 정보 없음」이 반입 건수만큼 부풀어 분포를 오염시킨다.
  const genreCounts = await db
    .select({ genre: Documents.genre, n: count() })
    .from(Documents)
    .where(eq(Documents.kind, 'sampled'))
    .groupBy(Documents.genre);
  const byGenre = new Map(genreCounts.map((g) => [g.genre ?? 'unclassified', g.n]));
  const known = GENRES.filter((g) => byGenre.has(g.key)).map((g) => ({ key: g.key, name: g.name, count: byGenre.get(g.key) ?? 0 }));
  const rest = [...byGenre]
    .filter(([key]) => GENRES.every((g) => g.key !== key))
    .map(([key, n]) => ({ key, name: key === 'unclassified' ? '장르 정보 없음' : key, count: n }));

  return {
    genres: [...known, ...rest],
    samplings,
    documents: documents.map((d) => ({
      ...d,
      createdAt: d.createdAt.toISOString(),
      runs: runCount.get(d.id) ?? 0,
      rounds: roundsOf.get(d.id) ?? [],
    })),
    promptSets: sets
      .filter((s) => generationById(s.generationId)?.status === 'active')
      .map((s) => ({ id: s.id, label: s.label, generationId: s.generationId })),
  };
};

export const actions: Actions = {
  // 표집 — 실서비스에서 공개 원고를 골라 문서로 들인다. 실행과 다른 축이라 runs에 남지 않는다.
  sample: async ({ request, platform }) => {
    if (!platform) error(500, 'platform unavailable');
    const form = await request.formData();
    const result = await spawnSampling(createDb(platform.env.DB), platform.env, Math.floor(Number(form.get('size') ?? 0)));
    return 'error' in result ? fail(400, { message: result.error }) : { sampled: true };
  },

  // 지목 반입 — 실서비스에서 refId로 원고를 가져온다. 표집이 지나는 공개 관문을 거치지 않으므로
  // 비공개 글도 들어온다. 그래서 반입분은 kind로 갈라 두고 라운드 후보에서 뺀다.
  intake: async ({ request, platform }) => {
    if (!platform) error(500, 'platform unavailable');
    const db = createDb(platform.env.DB);
    const form = await request.formData();
    const refIds = String(form.get('refIds') ?? '')
      .split(/[\s,]+/)
      .map((s) => s.trim())
      .filter(Boolean);
    if (refIds.length === 0) return fail(400, { message: '문서 식별자가 필요합니다' });

    const existing = await inChunks(refIds, (chunk) =>
      db.select({ refId: Documents.refId }).from(Documents).where(inArray(Documents.refId, chunk)),
    );
    const known = new Set(existing.map((e) => e.refId));
    const wanted = refIds.filter((id) => !known.has(id));
    if (wanted.length === 0) return { intake: { accepted: 0, reused: refIds.length, rejected: [] as string[] } };

    const api = createInternalApi(platform.env.INTERNAL_API_BASE, platform.env.INTERNAL_API_KEY);
    const extracted = await api.extract(wanted);

    const accepted: string[] = [];
    const rejected: string[] = [];
    for (const doc of extracted) {
      if (!doc.prose?.trim()) {
        rejected.push(doc.documentId);
        continue;
      }
      await db.insert(Documents).values({
        id: nanoid(),
        refId: doc.documentId,
        content: doc.prose,
        characterCount: doc.prose.length,
        kind: 'intake',
      });
      accepted.push(doc.documentId);
    }
    return { intake: { accepted: accepted.length, reused: known.size, rejected } };
  },

  run: async ({ request, platform }) => {
    if (!platform) error(500, 'platform unavailable');
    const form = await request.formData();
    const promptSetId = String(form.get('promptSetId') ?? '');
    const documentIds = form.getAll('documentIds').map(String);

    const result = await spawnRuns(createDb(platform.env.DB), platform.env, { promptSetId, documentIds });
    if ('error' in result) return fail(400, { message: result.error });
    // 걸어놓고 그 자리에 남으면 어디서 도는지 직접 찾아가야 한다 — 실행이 보이는 곳으로 보낸다.
    redirect(303, `/admin/runs?spawned=${result.spawned.length}&failed=${result.failed.length}`);
  },

  remove: async ({ request, platform }) => {
    if (!platform) error(500, 'platform unavailable');
    const db = createDb(platform.env.DB);
    const form = await request.formData();
    const id = String(form.get('id') ?? '');
    const [used] = await db.select({ id: Runs.id }).from(Runs).where(eq(Runs.documentId, id)).limit(1);
    if (used) return fail(400, { message: '실행이 걸린 문서는 지울 수 없습니다' });
    await db.delete(Documents).where(eq(Documents.id, id));
    return { removed: true };
  },
};
