// cspell:ignore ZWSP

import { and, asc, desc, eq } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { isRejectedResult, pickRounds } from '../feedback/rounds.ts';
import { buildModelConfig, overridesFromConfig } from '../feedback/tiers.ts';
import { FeedbackSessions, ManuscriptVersions, Reviews, ThreadComments, Threads } from './db/index.ts';
import { fetchManuscript } from './ingest.ts';
import { createInternalApi } from './internal-api.ts';
import {
  cancelWorkflow,
  fetchCatalog,
  fetchWorkflowFile,
  newPrismWorkflowId,
  PrismApiError,
  retryWorkflow,
  startWorkflow,
} from './prism.ts';
import type { BatchItem } from 'drizzle-orm/batch';
import type { AppCatalog, ModelConfig, RereviewTier, TierName, TierOverrides } from '../feedback/tiers.ts';
import type { Anchor, ManuscriptMeta, Pass, PreviousInput, PreviousThread } from '../feedback/types.ts';
import type { Db } from './db/index.ts';

type Env = App.Platform['env'];

// 글자 수는 typie 정본 정의를 따른다 — ZWSP 제거 후 grapheme cluster, 공백 포함. editor-resource/src/character_count.rs
export const countChars = (content: string): number => [...new Intl.Segmenter().segment(content.replaceAll('\u{200B}', ''))].length;

export const buildStartRows = (input: {
  refId: string;
  email: string;
  title: string | null;
  subtitle: string | null;
  content: string;
  prismWorkflowId: string;
  now: Date;
  catalog: AppCatalog;
  tier?: TierName;
  overrides?: TierOverrides;
}) => {
  const sessionId = nanoid();
  // 티어 확정은 여기 한 곳이다 — 행에 저장하는 값과 워크플로 이름이 갈라지지 않는다.
  const tier = input.tier ?? 'high';
  return {
    session: { id: sessionId, refId: input.refId, title: input.title, testerEmail: input.email, createdAt: input.now },
    version: {
      sessionId,
      version: 1,
      content: input.content,
      title: input.title,
      subtitle: input.subtitle,
      charCount: countChars(input.content),
      importedAt: input.now,
    },
    review: {
      sessionId,
      round: 1,
      prismWorkflowId: input.prismWorkflowId,
      status: 'running' as const,
      manuscriptVersion: 1,
      startedAt: input.now,
      tier,
      modelConfig: buildModelConfig(input.catalog, tier, input.overrides),
    },
  };
};

export const startFeedbackSession = async (
  db: Db,
  env: Env,
  input: { refId: string; email: string; catalog: AppCatalog; tier?: TierName; overrides?: TierOverrides },
): Promise<{ sessionId: string } | { error: string }> => {
  const api = createInternalApi(env.INTERNAL_API_BASE, env.INTERNAL_API_KEY);
  const manuscript = await fetchManuscript(api, input.refId);
  if ('error' in manuscript) return manuscript;

  const rows = buildStartRows({ ...input, ...manuscript, prismWorkflowId: newPrismWorkflowId(), now: new Date() });
  await db.batch([
    db.insert(FeedbackSessions).values(rows.session),
    db.insert(ManuscriptVersions).values(rows.version),
    db.insert(Reviews).values(rows.review),
  ]);

  try {
    await startWorkflow(env, {
      workflowId: rows.review.prismWorkflowId,
      workflow: rows.review.tier,
      input: {
        manuscriptPath: 'manuscript/v1.txt',
        meta: { title: manuscript.title, subtitle: manuscript.subtitle },
        // sparse — 무오버라이드 리뷰는 prism 기본값을 따른다(키 자체를 싣지 않는다)
        ...(input.overrides && Object.keys(input.overrides).length > 0 && { overrides: input.overrides }),
      },
      files: [{ path: 'manuscript/v1.txt', content: manuscript.content }],
    });
  } catch (err) {
    // 구 run-service 관례 승계: 시작 실패는 행에 귀속하고 화면이 사유를 보여 준다
    await db
      .update(Reviews)
      .set({ status: 'failed', error: String(err).slice(0, 1000), finishedAt: new Date() })
      .where(and(eq(Reviews.sessionId, rows.session.id), eq(Reviews.round, 1)));
  }
  return { sessionId: rows.session.id };
};

export const requestCancel = async (env: Env, prismWorkflowId: string): Promise<void> => {
  await cancelWorkflow(env, prismWorkflowId);
};

// 실패한 최신 회차를 멈춘 지점부터 이어간다 — 입력 파일·아티팩트는 prism에 이미 있어 재업로드가 없다.
// 취소 회차는 대상이 아니다: prism retry가 failed 종결만 수용한다(core/do.ts retryWorkflow).
export const resumeReview = async (db: Db, env: Env, sessionId: string): Promise<{ round: number } | { error: string }> => {
  const rounds = await db.select().from(Reviews).where(eq(Reviews.sessionId, sessionId)).orderBy(asc(Reviews.round));
  const latest = rounds.at(-1);
  if (!latest || latest.status !== 'failed') return { error: '지금은 이어서 다시 시도할 수 없어요' };

  try {
    await retryWorkflow(env, latest.prismWorkflowId);
  } catch (err) {
    if (err instanceof PrismApiError && err.code === 'retry-rejected') {
      // prism은 이미 failed가 아니다 — 이전 재개가 행 갱신 전에 죽은 잔재가 대부분이다. 행만 되돌려 두면
      // 다음 로드의 projectIfTerminal이 실제 상태로 수렴시킨다.
    } else if (err instanceof PrismApiError && err.code === 'retry-unsettled') {
      return { error: '아직 이어서 다시 시도할 준비가 안 됐어요. 잠시 후 다시 시도해 주세요' };
    } else if (err instanceof PrismApiError && err.status === 404) {
      // 시작 자체가 실패해 prism에 워크플로가 없다 — 이어갈 지점이 없으니 새 세션 경로로 안내한다.
      return { error: '이어서 다시 시도할 수 없는 실패예요. 새 세션으로 처음부터 시작해 주세요' };
    } else {
      return { error: '다시 시도 요청을 보내지 못했어요. 잠시 후 다시 시도해 주세요' };
    }
  }

  // 종결 산출물 컬럼을 눕혀 실행 중으로 되돌린다 — 종결 때 사영이 전부 다시 굳힌다(project.ts). failed 조건부라
  // 중복 제출·경합은 첫 갱신만 닿고, startedAt은 원 시작 시각 그대로다(실패~재개 공백은 화면이 파킹으로 감산).
  await db
    .update(Reviews)
    .set({ status: 'running', error: null, result: null, usage: null, events: null, questions: null, finishedAt: null })
    .where(and(eq(Reviews.sessionId, sessionId), eq(Reviews.round, latest.round), eq(Reviews.status, 'failed')));
  return { round: latest.round };
};

// 스레드는 상태 불문 전건이 실린다 — 무엇을 처분 대상으로 삼고 무엇을 억제로 접는지의 정책은 prism이 쥔다.
// 사영은 PREVIOUS 스키마가 연 필드만 통과시킨다: 스키마는 z.toJSONSchema 변환으로 전 object에
// additionalProperties: false를 달고 굳으므로, 행의 다른 컬럼이 하나라도 새면 워크플로 시작이 반려된다.
export const buildPreviousContext = (input: {
  threads: {
    id: string;
    pass: Pass;
    trait: string;
    body: string | null;
    anchors: Anchor[];
    state: PreviousThread['state'];
    issueId: string | null;
    comments: { author: 'tester' | 'ai'; body: string; createdAt: Date }[];
  }[];
  manuscriptPath: string;
  // base 회차가 입력을 읽은 시각 — 이보다 뒤의 답글이 "지난 리뷰가 응답하지 않은 새 답글"이다. prism은 시각을
  // 모르므로 판별은 여기서 끝내고 표지만 넘긴다.
  baseStartedAt: Date;
  meta: ManuscriptMeta;
}): PreviousInput => ({
  manuscriptPath: input.manuscriptPath,
  meta: { title: input.meta.title, subtitle: input.meta.subtitle },
  threads: input.threads.map((thread) => ({
    id: thread.id,
    pass: thread.pass,
    trait: thread.trait,
    body: thread.body ?? '',
    anchors: thread.anchors.map(({ head, tail }) => ({ head, tail })),
    // AI 코멘트는 지난 리뷰 자신의 산출물이다 — 작가의 반론만 컨텍스트에 싣는다.
    replies: thread.comments
      .filter((comment) => comment.author === 'tester')
      .map((comment) => ({ body: comment.body, fresh: comment.createdAt.getTime() > input.baseStartedAt.getTime() })),
    state: thread.state,
    // issue는 min(1) optional이다 — 값이 없으면 키 자체를 세우지 않는다(빈 문자열도 undefined도 반려 사유다).
    ...(thread.issueId !== null && thread.issueId !== '' && { issue: thread.issueId }),
  })),
});

export const startRereview = async (db: Db, env: Env, sessionId: string): Promise<{ round: number } | { error: string }> => {
  const rounds = await db.select().from(Reviews).where(eq(Reviews.sessionId, sessionId)).orderBy(asc(Reviews.round));
  if (rounds.length === 0) return { error: '리뷰가 없는 세션이에요' };
  // 거부 회차는 재검토의 기반이 못 된다 — 표지를 얹어 pickRounds가 강등·차단하게 한다.
  const picked = pickRounds(rounds.map((row) => ({ ...row, rejected: isRejectedResult(row.result) })));
  if (!picked.canRereview) return { error: '지금은 리뷰를 다시 요청할 수 없어요' };
  const base = picked.display;
  // canRereview가 REREVIEW_TIERS 밖 티어를 이미 걸렀다 — 이 캐스트는 그 판정의 좁히기다.
  const tier = base.tier as RereviewTier;

  // 구세션 가드 — 재검토는 지난 회차의 맥락을 이어 쓴다. 파이프라인이 바뀌기 전의 세션에는 이어 쓸 기반이
  // 없다. 행을 세우기 전에 끊는다: 세우고 실패로 귀속하면 화면에 재개 가능한 실패로 서서 같은 반려를 반복한다.
  // 프로브는 티어별 신 파이프라인 완주의 판별 파일이다 — high·medium은 마지막 스텝이 쓰는 연속성 시드
  // (구 research 구성 medium에는 없어, previous를 모르는 구 prism이 떠 있는 동안의 medium 재리뷰도 여기서
  // 반려된다), low는 판정 산출물(구 critique 구성 low에는 없고, 그 세션의 스레드는 pass 어휘가 달라 prism
  // 시작 검증에도 걸린다).
  const GUARD_FILES: Record<RereviewTier, string> = {
    high: 'artifacts/continuity.yaml',
    medium: 'artifacts/continuity.yaml',
    low: 'artifacts/judgment.yaml',
  };
  try {
    if ((await fetchWorkflowFile(env, base.prismWorkflowId, GUARD_FILES[tier])) === null) {
      return { error: '이 세션은 이전 버전 리뷰라 다시 요청할 수 없어요. 새 피드백으로 시작해 주세요' };
    }
  } catch {
    // 산출물의 유무를 확인하지 못한 것이지 없다고 확인한 것이 아니다 — 구세션으로 단정하지 않고 재시도를 안내한다.
    return { error: '리뷰를 다시 시작하지 못했어요. 잠시 후 다시 시도해 주세요' };
  }

  // 카탈로그는 스냅샷 조립·구 스냅샷 provider 폴백의 재료다 — 못 걷으면 행을 만들기 전에 명시 실패로 끝낸다.
  let catalog: AppCatalog;
  try {
    catalog = await fetchCatalog(env);
  } catch {
    return { error: '리뷰 구성을 불러오지 못했어요. 잠시 후 다시 시도해 주세요' };
  }

  // 티어 설정은 회차를 넘어 승계된다 — 재리뷰는 같은 조건에서 다시 보는 것이라 모델 구성이 바뀌면 변인이 섞인다.
  const overrides = overridesFromConfig(catalog, base.modelConfig as ModelConfig | null);

  const [session] = await db.select().from(FeedbackSessions).where(eq(FeedbackSessions.id, sessionId)).limit(1);
  if (!session) return { error: '세션을 찾을 수 없어요' };

  const api = createInternalApi(env.INTERNAL_API_BASE, env.INTERNAL_API_KEY);
  const manuscript = await fetchManuscript(api, session.refId);
  if ('error' in manuscript) return manuscript;

  // 입력이 같으면 새 버전 행을 만들지 않는다 — 무변경 재리뷰는 기존 버전을 다시 가리킨다.
  const [latestVersion] = await db
    .select()
    .from(ManuscriptVersions)
    .where(eq(ManuscriptVersions.sessionId, sessionId))
    .orderBy(desc(ManuscriptVersions.version))
    .limit(1);
  const now = new Date();
  const sameInput =
    latestVersion.content === manuscript.content &&
    latestVersion.title === manuscript.title &&
    latestVersion.subtitle === manuscript.subtitle;
  const nextVersion = sameInput ? latestVersion.version : latestVersion.version + 1;

  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion -- rounds.length > 0은 위에서 확인했다
  const round = rounds.at(-1)!.round + 1;
  const prismWorkflowId = newPrismWorkflowId();
  const inserts: BatchItem<'sqlite'>[] = [
    db.insert(Reviews).values({
      sessionId,
      round,
      prismWorkflowId,
      status: 'running' as const,
      manuscriptVersion: nextVersion,
      startedAt: now,
      tier,
      modelConfig: buildModelConfig(catalog, tier, overrides),
    }),
  ];
  if (!sameInput) {
    inserts.unshift(
      db.insert(ManuscriptVersions).values({
        sessionId,
        version: nextVersion,
        content: manuscript.content,
        title: manuscript.title,
        subtitle: manuscript.subtitle,
        charCount: countChars(manuscript.content),
        importedAt: now,
      }),
    );
  }
  // 세션 제목은 최신 원고를 따라간다 — 회차 행과 한 batch라 목록 표시가 이번 회차의 제목과 갈라지지 않는다.
  inserts.push(db.update(FeedbackSessions).set({ title: manuscript.title }).where(eq(FeedbackSessions.id, sessionId)));
  try {
    await db.batch(inserts as [BatchItem<'sqlite'>, ...BatchItem<'sqlite'>[]]);
  } catch {
    // 아직 귀속할 행이 서지 않았으니 갱신도 없다 — 회차 유일성(PK)에 진 동시 요청은 반려로 끝난다.
    return { error: '리뷰를 다시 시작하지 못했어요. 잠시 후 다시 시도해 주세요' };
  }

  // 여기부터 실패는 running 행에 귀속된다 — 잠금은 running 삽입 순간 발동했고, 이후 읽기가 확정 입력이다.
  try {
    const threads = await db.select().from(Threads).where(eq(Threads.sessionId, sessionId));
    const comments = await db
      .select({
        threadId: ThreadComments.threadId,
        author: ThreadComments.author,
        body: ThreadComments.body,
        createdAt: ThreadComments.createdAt,
      })
      .from(ThreadComments)
      .innerJoin(Threads, eq(Threads.id, ThreadComments.threadId))
      .where(eq(Threads.sessionId, sessionId))
      .orderBy(asc(ThreadComments.createdAt));
    const commentsOf = new Map<string, typeof comments>();
    for (const comment of comments) commentsOf.set(comment.threadId, [...(commentsOf.get(comment.threadId) ?? []), comment]);

    const previousManuscriptPath = `manuscript/v${base.manuscriptVersion}.txt`;
    const [previousVersion] = await db
      .select()
      .from(ManuscriptVersions)
      .where(and(eq(ManuscriptVersions.sessionId, sessionId), eq(ManuscriptVersions.version, base.manuscriptVersion)))
      .limit(1);
    const previous = buildPreviousContext({
      threads: threads.map((thread) => ({
        id: thread.id,
        pass: thread.pass,
        trait: thread.trait,
        body: thread.body,
        anchors: thread.anchors as Anchor[],
        state: thread.state,
        issueId: thread.issueId,
        comments: commentsOf.get(thread.id) ?? [],
      })),
      manuscriptPath: previousManuscriptPath,
      baseStartedAt: base.startedAt,
      meta: { title: previousVersion.title, subtitle: previousVersion.subtitle },
    });

    // 이전 회차 산출물 시딩 — 자기 이전 산출물을 edit로 고쳐 내는 스테이지 전부가 원래 경로 그대로 실리고,
    // 하나라도 없으면 시작 실패(prism의 prepare 스텝도 재검하지만 여기서 먼저 끊는다). delivery.yaml은 싣지
    // 않는다 — prism의 프로그램 소비가 없고, 시드가 자리에 서면 delivery가 지난 총평을 통독·edit 재작성해
    // 턴이 폭증한다(2026-08-14 비용 감사, 스펙 §3). medium은 high의 감산이라 기술 5종만 싣고 해석·기준표·판정
    // 시드가 빠지며(prism plans/2026-08-17-feedback-medium.md Task 8), low의 재검토가 소비하는 이전 맥락은
    // previous 입력(스레드는 프로그램이 previous/threads.yaml로 렌더)과 이전 원고 파일뿐이라 시딩이 없다.
    const DESCRIPTION_ARTIFACTS = [
      'artifacts/movements.yaml',
      'artifacts/narration.yaml',
      'artifacts/audience.yaml',
      'artifacts/condition.yaml',
      'artifacts/experience.yaml',
    ];
    const TIER_ARTIFACTS: Record<RereviewTier, string[]> = {
      high: [...DESCRIPTION_ARTIFACTS, 'artifacts/interpretation.yaml', 'artifacts/rubric.yaml', 'artifacts/judgment.yaml'],
      medium: DESCRIPTION_ARTIFACTS,
      low: [],
    };
    const ARTIFACTS = TIER_ARTIFACTS[tier];
    // 연속성 시드만 에이전트 비가시 경로로 옮겨 싣는다 — 프로그램이 병합한 완전본(승계 격상·범위 밖 기록의
    // 처분)이라 에이전트 산출물에는 없고, 컨텍스트에 서면 지난 지적의 전사 압력이 되살아난다. low는 연속성
    // 산출 자체가 없다.
    const PROGRAM_SEEDS = tier === 'low' ? [] : [{ from: 'artifacts/continuity.yaml', to: 'previous/continuity.yaml' }];
    const files: { path: string; content: string }[] = [];
    for (const path of ARTIFACTS) {
      const content = await fetchWorkflowFile(env, base.prismWorkflowId, path);
      if (content === null) throw new Error(`previous artifact missing: ${path}`);
      files.push({ path, content });
    }
    for (const seed of PROGRAM_SEEDS) {
      const content = await fetchWorkflowFile(env, base.prismWorkflowId, seed.from);
      if (content === null) throw new Error(`previous artifact missing: ${seed.from}`);
      files.push({ path: seed.to, content });
    }
    files.push({ path: previousManuscriptPath, content: previousVersion.content });
    const manuscriptPath = `manuscript/v${nextVersion}.txt`;
    // 무변경 재리뷰(nextVersion === base.manuscriptVersion)면 같은 파일이 두 경로 항목으로 서지 않게 중복을 거른다.
    if (manuscriptPath !== previousManuscriptPath) files.push({ path: manuscriptPath, content: manuscript.content });

    await startWorkflow(env, {
      workflowId: prismWorkflowId,
      workflow: tier,
      // sparse — 승계할 오버라이드가 없으면 키 자체를 싣지 않는다(startFeedbackSession과 같은 관례).
      input: {
        manuscriptPath,
        meta: { title: manuscript.title, subtitle: manuscript.subtitle },
        previous,
        ...(Object.keys(overrides).length > 0 && { overrides }),
      },
      files,
    });
  } catch (err) {
    await db
      .update(Reviews)
      .set({ status: 'failed', error: String(err).slice(0, 1000), finishedAt: new Date() })
      .where(and(eq(Reviews.sessionId, sessionId), eq(Reviews.round, round), eq(Reviews.status, 'running')));
    // 행은 failed로 귀속해 두고 호출부에는 실패로 답한다 — 성공을 돌려주면 화면이 시작 토스트를 띄운 뒤
    // 곧바로 실패 배너를 세워 스스로를 뒤집는다.
    return { error: '리뷰를 다시 시작하지 못했어요. 잠시 후 다시 시도해 주세요' };
  }
  return { round };
};
