import { error, fail } from '@sveltejs/kit';
import { and, asc, eq } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { canClose, canReopen } from '$lib/feedback/threads.ts';
import { isAdmin } from '$lib/server/auth.ts';
import { createDb, FeedbackSessions, ManuscriptVersions, Reactions, Reviews, ThreadComments, Threads } from '$lib/server/db/index.ts';
import { projectIfTerminal } from '$lib/server/project.ts';
import { requestCancel } from '$lib/server/reviews.ts';
import type { SseEvent } from '$lib/feedback/sse.ts';
import type { Anchor, FeedbackResult } from '$lib/feedback/types.ts';
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

export const load: PageServerLoad = async ({ params, locals, platform }) => {
  if (!platform) error(500, 'platform unavailable');
  const env = platform.env;
  const db = createDb(env.DB);

  const session = await ownedSession(db, env, params.id, locals.email);

  // Phase 1은 세션당 리뷰가 round 1 하나뿐이다.
  const [loaded] = await db.select().from(Reviews).where(eq(Reviews.sessionId, params.id)).limit(1);
  if (!loaded) error(404, 'not found');

  let review = loaded;
  if (loaded.status === 'running') {
    try {
      await projectIfTerminal(db, env, loaded);
    } catch (err) {
      // 사영 실패는 화면을 막지 않는다 — running으로 계속 표시하고 다음 로드가 재시도한다.
      // 무음이면 실패 반복을 D1 대조로만 알 수 있으므로 로그는 남긴다(wrangler tail로 관측).
      console.error('projection failed', loaded.prismSessionId, err);
    }
    const [projected] = await db.select().from(Reviews).where(eq(Reviews.sessionId, params.id)).limit(1);
    if (projected) review = projected;
  }

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
      startedAt: review.startedAt.getTime(),
      finishedAt: review.finishedAt?.getTime() ?? null,
      error: review.error,
      // 두 json 컬럼의 생산자는 사영뿐이다 — 쓴 타입 그대로 좁힌다(project.ts:84-97).
      result: review.result as FeedbackResult | null,
      // 스냅샷은 실패·취소 타임라인의 원천이다. 완료 화면은 쓰지 않으므로 싣지 않는다(과정 보기는 자기 라우트에서 읽는다).
      events: review.status === 'completed' ? null : (review.events as SseEvent[] | null),
    },
    threads: threads.map((thread) => ({
      id: thread.id,
      issueIndex: thread.issueIndex,
      axis: thread.axis,
      pass: thread.pass,
      body: thread.body,
      anchors: thread.anchors as Anchor[],
      state: thread.state,
    })),
    comments: comments.map((comment) => ({ ...comment, createdAt: comment.createdAt.getTime() })),
    reaction: reaction ? { value: reaction.value, note: reaction.note } : null,
    email: locals.email,
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
      await requestCancel(env, review.prismSessionId);
    } catch {
      return fail(502, { error: '중단 요청을 보내지 못했어요. 잠시 후 다시 시도해 주세요' });
    }
    return { canceled: true };
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
};
