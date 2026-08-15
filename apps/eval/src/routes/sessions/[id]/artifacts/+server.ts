import { error, json } from '@sveltejs/kit';
import { asc, eq } from 'drizzle-orm';
import { ARTIFACT_ORDER, ARTIFACT_PATHS, parseArtifact } from '$lib/feedback/artifacts.ts';
import { isRejectedResult, pickRounds } from '$lib/feedback/rounds.ts';
import { isAdmin } from '$lib/server/auth.ts';
import { createDb, FeedbackSessions, Reviews } from '$lib/server/db/index.ts';
import { fetchWorkflowFile } from '$lib/server/prism.ts';
import type { Artifacts } from '$lib/feedback/artifacts.ts';
import type { RequestHandler } from './$types';

// 산출물은 결과 화면이 여는 회차의 것이다 — 표시 회차 규칙은 pickRounds 한 곳이 정한다(과정 보기와 같은 잣대).
// 회차·워크플로 id는 클라이언트 입력이 아니다.
export const GET: RequestHandler = async ({ params, locals, platform }) => {
  if (!platform) error(500, 'platform unavailable');
  const env = platform.env;
  const db = createDb(env.DB);

  const [session] = await db.select().from(FeedbackSessions).where(eq(FeedbackSessions.id, params.id)).limit(1);
  if (!session) error(404, 'not found');
  if (session.testerEmail !== locals.email && !isAdmin(env, locals.email)) error(403, 'forbidden');

  const rows = await db.select().from(Reviews).where(eq(Reviews.sessionId, params.id)).orderBy(asc(Reviews.round));
  if (rows.length === 0) error(404, 'not found');
  const review = pickRounds(rows.map((row) => ({ ...row, rejected: isRejectedResult(row.result) }))).display;

  // 정리 UI는 high 파이프라인의 계약을 미러한다 — 다른 티어는 같은 이름의 파일도 형태가 달라 열지 않는다.
  if (review.status !== 'completed' || review.rejected || review.tier !== 'high') error(409, 'unavailable');

  // 하나라도 못 걷으면 전체 실패다 — 부분 화면 대신 화면의 재시도가 경로다.
  let contents: (string | null)[];
  try {
    contents = await Promise.all(ARTIFACT_ORDER.map((name) => fetchWorkflowFile(env, review.prismWorkflowId, ARTIFACT_PATHS[name])));
  } catch (err) {
    console.error('artifacts fetch failed', review.prismWorkflowId, err);
    error(502, 'upstream');
  }

  const artifacts = Object.fromEntries(ARTIFACT_ORDER.map((name, i) => [name, parseArtifact(name, contents[i])])) as Artifacts;
  return json({ artifacts }, { headers: { 'cache-control': 'no-store' } });
};
