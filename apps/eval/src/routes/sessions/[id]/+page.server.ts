import { error, fail } from '@sveltejs/kit';
import { and, asc, eq } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { askAnswerIndex } from '$lib/feedback/questions.ts';
import { canClose, canReopen } from '$lib/feedback/threads.ts';
import { isAdmin } from '$lib/server/auth.ts';
import { createDb, FeedbackSessions, ManuscriptVersions, Reactions, Reviews, ThreadComments, Threads } from '$lib/server/db/index.ts';
import { getWorkflowInvocations, PrismApiError, resolveAskUser } from '$lib/server/prism.ts';
import { projectIfTerminal, seedEvents } from '$lib/server/project.ts';
import { collectAskAnswers } from '$lib/server/questions.ts';
import { requestCancel } from '$lib/server/reviews.ts';
import type { AskAnswer } from '$lib/feedback/live.ts';
import type { SseEvent } from '$lib/feedback/sse.ts';
import type { ModelConfig } from '$lib/feedback/tiers.ts';
import type { Anchor, FeedbackResult, ReviewQuestionRecord } from '$lib/feedback/types.ts';
import type { Db } from '$lib/server/db/index.ts';
import type { Actions, PageServerLoad } from './$types';

type Env = App.Platform['env'];

const ownedSession = async (db: Db, env: Env, sessionId: string, email: string) => {
  const [session] = await db.select().from(FeedbackSessions).where(eq(FeedbackSessions.id, sessionId)).limit(1);
  if (!session) error(404, 'not found');
  if (session.testerEmail !== email && !isAdmin(env, email)) error(403, 'forbidden');
  return session;
};

// D1은 FK를 강제하지 않는다 — threadId가 이 세션 소속인지는 매 액션이 직접 확인한다.
const sessionThread = async (db: Db, sessionId: string, threadId: string) => {
  const [thread] = await db
    .select()
    .from(Threads)
    .where(and(eq(Threads.id, threadId), eq(Threads.sessionId, sessionId)))
    .limit(1);
  if (!thread) error(404, 'not found');
  return thread;
};

// 답변은 카드가 JSON 문자열로 보내는 클라이언트 입력이다 — prism 계약(AskAnswer[])의 형태는 이 경계가 확인한다.
// 빈 답(질문 0개·선택 0개)은 파킹을 풀지 못하므로 형태 위반과 같이 막는다.
const isAskAnswers = (value: unknown): value is AskAnswer[] =>
  Array.isArray(value) &&
  value.length > 0 &&
  value.every(
    (answer) =>
      typeof answer === 'object' &&
      answer !== null &&
      typeof (answer as { question?: unknown }).question === 'string' &&
      Array.isArray((answer as { choice?: unknown }).choice) &&
      (answer as { choice: unknown[] }).choice.length > 0 &&
      (answer as { choice: unknown[] }).choice.every((choice) => typeof choice === 'string'),
  );

export const load: PageServerLoad = async ({ params, locals, platform }) => {
  if (!platform) error(500, 'platform unavailable');
  const env = platform.env;
  const db = createDb(env.DB);

  const session = await ownedSession(db, env, params.id, locals.email);
  const admin = isAdmin(env, locals.email);

  // Phase 1은 세션당 리뷰가 round 1 하나뿐이다.
  const [loaded] = await db.select().from(Reviews).where(eq(Reviews.sessionId, params.id)).limit(1);
  if (!loaded) error(404, 'not found');

  let review = loaded;
  let liveEvents: SseEvent[] | null = null;
  let askAnswers: Record<string, AskAnswer[]> | null = null;
  if (loaded.status === 'running') {
    try {
      await projectIfTerminal(db, env, loaded);
    } catch (err) {
      // 사영 실패는 화면을 막지 않는다 — running으로 계속 표시하고 다음 로드가 재시도한다.
      // 무음이면 실패 반복을 D1 대조로만 알 수 있으므로 로그는 남긴다(wrangler tail로 관측).
      console.error('projection failed', loaded.prismWorkflowId, err);
    }
    const [projected] = await db.select().from(Reviews).where(eq(Reviews.sessionId, params.id)).limit(1);
    if (projected) review = projected;
    if (review.status === 'running') {
      try {
        // 첫 화면 시드 — 없으면 클라이언트가 재생을 화면에서 재연해, 새로고침마다 과거 기록이 빨리감기로 보인다.
        liveEvents = await seedEvents(env, review.prismWorkflowId);
      } catch (err) {
        // 시드 실패도 화면을 막지 않는다 — 클라이언트 SSE 재생이 처음부터 채운다.
        console.error('event snapshot failed', review.prismWorkflowId, err);
      }
      if (liveEvents !== null) {
        try {
          // 답한 질문의 문면은 이벤트에 없다 — 원장에서 걷어 카드에 실어 준다. 종결 리뷰는 사영이 담당한다.
          askAnswers = await collectAskAnswers(env, liveEvents);
        } catch (err) {
          // 답변 조회 실패도 화면을 막지 않는다 — 답변함 카드가 문면 없이 서고 다음 로드가 재시도한다.
          console.error('ask answers fetch failed', review.prismWorkflowId, err);
        }
      }
    }
  }

  // 종결 리뷰의 답변 문면은 원장이 아니라 사영 기록이 원천이다(project.ts) — 실패·취소로 끝난 리뷰도 타임라인을
  // 그리므로 여기서 실어야 answered 카드가 문면 없이 서지 않는다(완료 화면은 타임라인을 그리지 않는다).
  if (review.status !== 'running') askAnswers = askAnswerIndex(review.questions as ReviewQuestionRecord[] | null);

  const [version] = await db
    .select()
    .from(ManuscriptVersions)
    .where(and(eq(ManuscriptVersions.sessionId, params.id), eq(ManuscriptVersions.version, review.manuscriptVersion)))
    .limit(1);
  if (!version) error(500, 'manuscript missing');

  const threads = await db.select().from(Threads).where(eq(Threads.sessionId, params.id)).orderBy(asc(Threads.issueIndex));

  // threadId 목록을 IN으로 묶으면 D1 문장당 바인딩 한도(100)에 걸린다 — 세션으로 조인해 한 문장으로 가져온다.
  const comments = await db
    .select({
      id: ThreadComments.id,
      threadId: ThreadComments.threadId,
      author: ThreadComments.author,
      body: ThreadComments.body,
      createdAt: ThreadComments.createdAt,
    })
    .from(ThreadComments)
    .innerJoin(Threads, eq(Threads.id, ThreadComments.threadId))
    .where(eq(Threads.sessionId, params.id))
    .orderBy(asc(ThreadComments.createdAt));

  const [reaction] = await db
    .select()
    .from(Reactions)
    .where(and(eq(Reactions.sessionId, params.id), eq(Reactions.reviewRound, review.round)))
    .limit(1);

  return {
    session: { id: session.id, refId: session.refId, title: session.title },
    version: { version: version.version, content: version.content, charCount: version.charCount },
    review: {
      round: review.round,
      status: review.status,
      tier: review.tier,
      startedAt: review.startedAt.getTime(),
      finishedAt: review.finishedAt?.getTime() ?? null,
      error: review.error,
      // 두 json 컬럼의 생산자는 사영뿐이다 — 쓴 타입 그대로 좁힌다(project.ts의 projectIfTerminal).
      result: review.result as FeedbackResult | null,
      // 스냅샷은 실패·취소 타임라인과 실행 중 첫 화면의 원천이다. 완료 화면은 쓰지 않으므로 싣지 않는다
      // (과정 보기는 자기 라우트에서 읽는다). 실행 중은 방금 걷은 백로그가, 종결은 사영된 events가 원천이다.
      events: review.status === 'completed' ? null : (liveEvents ?? (review.events as SseEvent[] | null)),
    },
    // 답한 질문 카드가 보여 줄 답변 — toolCallId로 색인한다. 실행 중에만 걷는다(종결 리뷰는 사영이 원천).
    askAnswers,
    threads: threads.map((thread) => ({
      id: thread.id,
      issueIndex: thread.issueIndex,
      axis: thread.axis,
      pass: thread.pass,
      body: thread.body,
      anchors: thread.anchors as Anchor[],
      state: thread.state,
      reaction: thread.reaction,
    })),
    comments: comments.map((comment) => ({ ...comment, createdAt: comment.createdAt.getTime() })),
    reaction: reaction ? { value: reaction.value, note: reaction.note } : null,
    email: locals.email,
    isAdmin: admin,
    // admin에게만 — 테스터 응답에는 데이터 자체를 싣지 않는다
    modelConfig: admin ? ((review.modelConfig as ModelConfig | null) ?? null) : null,
  };
};

export const actions: Actions = {
  cancel: async ({ params, locals, platform }) => {
    if (!platform) error(500, 'platform unavailable');
    const env = platform.env;
    const db = createDb(env.DB);

    await ownedSession(db, env, params.id, locals.email);

    const [review] = await db.select().from(Reviews).where(eq(Reviews.sessionId, params.id)).limit(1);
    if (!review || review.status !== 'running') return fail(409, { error: '이미 끝난 리뷰예요' });

    try {
      // 상태는 사영이 확정한다 — 여기서는 요청만 보낸다.
      await requestCancel(env, review.prismWorkflowId);
    } catch {
      return fail(502, { error: '중단 요청을 보내지 못했어요. 잠시 후 다시 시도해 주세요' });
    }
    return { canceled: true };
  },

  answer: async ({ params, request, locals, platform }) => {
    if (!platform) error(500, 'platform unavailable');
    const env = platform.env;
    const db = createDb(env.DB);

    await ownedSession(db, env, params.id, locals.email);

    const [review] = await db.select().from(Reviews).where(eq(Reviews.sessionId, params.id)).limit(1);
    if (!review || review.status !== 'running') return fail(409, { error: '이미 끝난 리뷰예요' });

    const form = await request.formData();
    const agentId = String(form.get('agentId') ?? '');
    const toolCallId = String(form.get('toolCallId') ?? '');
    let answers: unknown;
    try {
      answers = JSON.parse(String(form.get('answers') ?? ''));
    } catch {
      return fail(400, { error: '답변 형식이 잘못됐어요' });
    }
    if (!isAskAnswers(answers)) return fail(400, { error: '답변 형식이 잘못됐어요' });

    // agentId는 클라이언트 입력이다 — 이 리뷰의 워크플로 소속인지 확인해야 남의 세션 질문에 답하는 경로가 닫힌다.
    try {
      const invocations = await getWorkflowInvocations(env, review.prismWorkflowId);
      if (invocations.every((invocation) => invocation.agentId !== agentId)) return fail(403, { error: '이 리뷰의 질문이 아니에요' });
    } catch {
      return fail(502, { error: '답변을 보내지 못했어요. 잠시 후 다시 시도해 주세요' });
    }

    try {
      await resolveAskUser(env, agentId, toolCallId, answers);
    } catch (err) {
      // 취소가 이겨 run이 종결된 뒤의 제출도 하니스가 no-pending-tool로 답한다 — 같은 분기로 흡수된다.
      if (err instanceof PrismApiError && err.code === 'no-pending-tool') return fail(409, { error: '이미 답변된 질문이에요' });
      if (err instanceof PrismApiError && err.status === 400) return fail(400, { error: '답변을 보내지 못했어요. 다시 시도해 주세요' });
      return fail(502, { error: '답변을 보내지 못했어요. 잠시 후 다시 시도해 주세요' });
    }
    return { answered: true };
  },

  reply: async ({ params, request, locals, platform }) => {
    if (!platform) error(500, 'platform unavailable');
    const env = platform.env;
    const db = createDb(env.DB);

    await ownedSession(db, env, params.id, locals.email);

    const form = await request.formData();
    const threadId = String(form.get('threadId') ?? '');
    const body = String(form.get('body') ?? '').trim();
    if (body.length === 0) return fail(400, { error: '답글 내용을 입력해 주세요' });

    await sessionThread(db, params.id, threadId);
    await db.insert(ThreadComments).values({ id: nanoid(), threadId, author: 'tester', body, reviewRound: null, createdAt: new Date() });

    return { replied: true };
  },

  deleteReply: async ({ params, request, locals, platform }) => {
    if (!platform) error(500, 'platform unavailable');
    const env = platform.env;
    const db = createDb(env.DB);

    await ownedSession(db, env, params.id, locals.email);

    const form = await request.formData();
    const commentId = String(form.get('commentId') ?? '');
    // 코멘트의 세션 소속도 조인으로 확인한다 — sessionThread와 같은 이유(D1은 FK를 강제하지 않는다).
    const [comment] = await db
      .select({ id: ThreadComments.id, author: ThreadComments.author })
      .from(ThreadComments)
      .innerJoin(Threads, eq(Threads.id, ThreadComments.threadId))
      .where(and(eq(ThreadComments.id, commentId), eq(Threads.sessionId, params.id)))
      .limit(1);
    if (!comment) error(404, 'not found');
    // AI 댓글은 리뷰 산출물이다 — 지울 수 있는 것은 테스터 자신의 답글뿐.
    if (comment.author !== 'tester') return fail(403, { error: 'AI 댓글은 지울 수 없어요' });

    await db.delete(ThreadComments).where(eq(ThreadComments.id, commentId));
    return { deleted: true };
  },

  close: async ({ params, request, locals, platform }) => {
    if (!platform) error(500, 'platform unavailable');
    const env = platform.env;
    const db = createDb(env.DB);

    await ownedSession(db, env, params.id, locals.email);

    const form = await request.formData();
    const threadId = String(form.get('threadId') ?? '');
    const thread = await sessionThread(db, params.id, threadId);
    if (!canClose(thread.state)) return fail(409, { error: '이미 닫힌 스레드예요' });

    await db.update(Threads).set({ state: 'closed', stateChangedAt: new Date() }).where(eq(Threads.id, threadId));
    return { closed: true };
  },

  reopen: async ({ params, request, locals, platform }) => {
    if (!platform) error(500, 'platform unavailable');
    const env = platform.env;
    const db = createDb(env.DB);

    await ownedSession(db, env, params.id, locals.email);

    const form = await request.formData();
    const threadId = String(form.get('threadId') ?? '');
    const thread = await sessionThread(db, params.id, threadId);
    if (!canReopen(thread.state)) return fail(409, { error: '이미 열린 스레드예요' });

    await db.update(Threads).set({ state: 'open', stateChangedAt: new Date() }).where(eq(Threads.id, threadId));
    return { reopened: true };
  },

  react: async ({ params, request, locals, platform }) => {
    if (!platform) error(500, 'platform unavailable');
    const env = platform.env;
    const db = createDb(env.DB);

    await ownedSession(db, env, params.id, locals.email);

    const form = await request.formData();
    const value = String(form.get('value') ?? '');
    if (value !== 'up' && value !== 'down') return fail(400, { error: '반응을 골라 주세요' });
    const noted = String(form.get('note') ?? '').trim();
    const note = noted.length === 0 ? null : noted;

    // 회차는 서버가 정한다 — Phase 1은 세션당 리뷰가 하나뿐이지만 축은 리뷰 행을 따른다.
    const [review] = await db.select({ round: Reviews.round }).from(Reviews).where(eq(Reviews.sessionId, params.id)).limit(1);
    if (!review) error(404, 'not found');

    await db
      .insert(Reactions)
      .values({ sessionId: params.id, reviewRound: review.round, value, note, createdAt: new Date() })
      .onConflictDoUpdate({ target: [Reactions.sessionId, Reactions.reviewRound], set: { value, note } });

    return { reacted: true };
  },

  // 해제는 별도 액션이다 — react가 "같은 값이면 해제"로 추론하면 같은 값을 다시 보내는 메모 폼이
  // 반응을 지운다. 설정·해제를 나누면 연타도 멱등이다(선택된 버튼만 클라이언트가 해제로 보낸다).
  unreact: async ({ params, locals, platform }) => {
    if (!platform) error(500, 'platform unavailable');
    const env = platform.env;
    const db = createDb(env.DB);

    await ownedSession(db, env, params.id, locals.email);

    const [review] = await db.select({ round: Reviews.round }).from(Reviews).where(eq(Reviews.sessionId, params.id)).limit(1);
    if (!review) error(404, 'not found');

    // 행 삭제라 한 줄 메모도 함께 사라진다 — 메모 입력 밴드 자체가 반응에 종속이다(반응 없음 = 밴드 접힘).
    await db.delete(Reactions).where(and(eq(Reactions.sessionId, params.id), eq(Reactions.reviewRound, review.round)));
    return { unreacted: true };
  },

  reactThread: async ({ params, request, locals, platform }) => {
    if (!platform) error(500, 'platform unavailable');
    const env = platform.env;
    const db = createDb(env.DB);

    await ownedSession(db, env, params.id, locals.email);

    const form = await request.formData();
    const threadId = String(form.get('threadId') ?? '');
    const value = String(form.get('value') ?? '');
    if (value !== 'up' && value !== 'down') return fail(400, { error: '반응을 골라 주세요' });

    await sessionThread(db, params.id, threadId);
    await db.update(Threads).set({ reaction: value }).where(eq(Threads.id, threadId));
    return { reacted: true };
  },

  unreactThread: async ({ params, request, locals, platform }) => {
    if (!platform) error(500, 'platform unavailable');
    const env = platform.env;
    const db = createDb(env.DB);

    await ownedSession(db, env, params.id, locals.email);

    const form = await request.formData();
    const threadId = String(form.get('threadId') ?? '');
    await sessionThread(db, params.id, threadId);
    await db.update(Threads).set({ reaction: null }).where(eq(Threads.id, threadId));
    return { unreacted: true };
  },
};
