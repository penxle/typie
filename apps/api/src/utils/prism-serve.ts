import { logger } from '@typie/lib';
import { PrismToolResolver, SiteState } from '@typie/lib/enums';
import { serveVerdict, TOOL_META, toolFailure } from '@typie/prism';
import { and, asc, eq } from 'drizzle-orm';
import { redis } from '#/cache.ts';
import { db, first, PrismRuns, PrismSessions, Sites } from '#/db/index.ts';
import { env } from '#/env.ts';
import { prism, PrismApiError } from '#/external/prism.ts';
import { pubsub } from '#/pubsub.ts';
import { prismNotificationUserActionKey, prismUserActionNotification } from './prism-notification.ts';
import { pushCopy, pushKey, shouldPushAsk } from './prism-push-core.ts';
import { recordToolResolution, withToolLedger } from './prism-tool-calls.ts';
import { DENIED_MESSAGE, ERROR_MESSAGE } from './prism-tool-messages.ts';
import { prismPreflights, prismTools } from './prism-tools.ts';
import type { AgentState, ToolFailure } from '@typie/prism';

const log = logger.getChild('prism-serve');

export const runSite = async (session: { id: string; userId: string }, runSeq: number | null): Promise<string | null> => {
  if (runSeq !== null) {
    const run = await db
      .select({ siteId: PrismRuns.siteId })
      .from(PrismRuns)
      .where(and(eq(PrismRuns.sessionId, session.id), eq(PrismRuns.runSeq, runSeq)))
      .then(first);
    if (run?.siteId) return run.siteId;
  }

  const fallback = await db
    .select({ id: Sites.id })
    .from(Sites)
    .where(and(eq(Sites.userId, session.userId), eq(Sites.state, SiteState.ACTIVE)))
    .orderBy(asc(Sites.createdAt))
    .limit(1)
    .then(first);

  return fallback?.id ?? null;
};

const pendingAgent = async (agentId: string, toolCallId: string): Promise<AgentState | null> => {
  let agent;
  try {
    agent = await prism.getAgent(agentId);
  } catch (err) {
    if (err instanceof PrismApiError && err.status === 404) return null;
    throw err;
  }

  return agent.pending && agent.pending.toolCallId === toolCallId ? agent : null;
};

const pushAsk = async (
  session: { id: string; userId: string; title: string | null },
  ask: { toolCallId: string; tool: string; data: unknown; at: number },
): Promise<void> => {
  const { PUSH_TTL_SECONDS, sendPushNotificationOnce } = await import('#/external/firebase.ts');
  if (Date.now() - ask.at > PUSH_TTL_SECONDS * 1000) return;

  const { title, body } = pushCopy(ask.tool, ask.data, session.title);

  const delivery = await sendPushNotificationOnce({
    key: pushKey.ask(ask.toolCallId),
    userId: session.userId,
    title,
    body,
    link: `${env.WEBSITE_URL}/initial?prism=${session.id}`,
  });

  if (delivery === 'failed') throw new Error(`prism ask push failed for ${ask.toolCallId}`);
};

const notifyAsk = async (
  session: { id: string; userId: string; title: string | null },
  ask: { agentId: string; toolCallId: string; tool: string; data: unknown; at: number; startedAt: number },
): Promise<void> => {
  const rawUserActionAt = await redis.get(prismNotificationUserActionKey(ask.agentId)).catch((err) => {
    log.warn('prism notification action timestamp unavailable: {agentId} {*}', { agentId: ask.agentId, error: err });
    return null;
  });

  pubsub.publish(
    'prism:notification',
    session.userId,
    prismUserActionNotification({
      sessionId: session.id,
      toolCallId: ask.toolCallId,
      startedAt: ask.startedAt,
      lastUserActionAt: rawUserActionAt === null ? undefined : Number(rawUserActionAt),
      requestedAt: ask.at,
    }),
  );
  await pushAsk(session, ask);
};

type PreflightOutcome = { state: 'gone' | 'clear' } | { state: 'failed'; failure: ToolFailure };

const preflight = async (
  session: typeof PrismSessions.$inferSelect,
  args: { agentId: string; runSeq: number | null; toolCallId: string; tool: string; input: unknown },
): Promise<PreflightOutcome> => {
  const agent = await pendingAgent(args.agentId, args.toolCallId);
  if (agent === null) return { state: 'gone' };

  const preverify = prismPreflights[args.tool];
  if (preverify === undefined) return { state: 'clear' };

  const siteId = await runSite(session, args.runSeq);
  if (siteId === null) return { state: 'clear' };

  try {
    const base = { userId: session.userId, session, siteId, toolCallId: args.toolCallId, agent, afterCommit: undefined };
    const failure = await preverify({ ...base, executor: db }, args.input);
    return failure === null ? { state: 'clear' } : { state: 'failed', failure };
  } catch (err) {
    log.warn('prism tool preflight failed: {tool} {toolCallId} {*}', { tool: args.tool, toolCallId: args.toolCallId, error: err });
    return { state: 'clear' };
  }
};

export const serveTool = async (args: {
  sessionId: string;
  agentId: string;
  origin: { kind: 'run'; runSeq: number | null } | { kind: 'workflow'; startedAt: number };
  toolCallId: string;
  tool: string;
  input: unknown;
  at: number;
}): Promise<void> => {
  const session = await db.select().from(PrismSessions).where(eq(PrismSessions.id, args.sessionId)).then(first);
  if (!session || session.deletedAt !== null) return;

  const runSeq = args.origin.kind === 'run' ? args.origin.runSeq : null;

  const call = { toolCallId: args.toolCallId, tool: args.tool, resolver: PrismToolResolver.SERVER };
  const verdict = serveVerdict(args.tool, session.toolPolicy);
  if (verdict === null) {
    const outcome = await preflight(session, { ...args, runSeq });
    if (outcome.state === 'gone') return;

    if (outcome.state === 'failed') {
      await recordToolResolution(session, call, outcome.failure);
      await prism.resolveTool(args.agentId, args.toolCallId, outcome.failure);
      return;
    }

    if (!shouldPushAsk(args.tool, session.toolPolicy)) return;

    const notificationRunSeq = args.origin.kind === 'run' ? (args.origin.runSeq ?? session.openRunSeq) : null;
    const run =
      notificationRunSeq === null
        ? null
        : await db
            .select({ startedAt: PrismRuns.startedAt })
            .from(PrismRuns)
            .where(and(eq(PrismRuns.sessionId, session.id), eq(PrismRuns.runSeq, notificationRunSeq)))
            .then(first);

    await notifyAsk(session, {
      agentId: args.agentId,
      toolCallId: args.toolCallId,
      tool: args.tool,
      data: args.input,
      at: args.at,
      startedAt: args.origin.kind === 'workflow' ? args.origin.startedAt : (run?.startedAt.valueOf() ?? args.at),
    });
    return;
  }

  const agent = await pendingAgent(args.agentId, args.toolCallId);
  if (agent === null) return;

  let result: unknown;
  if (verdict === 'deny') {
    result = toolFailure('denied', DENIED_MESSAGE);
    await recordToolResolution(session, call, result);
  } else {
    const handler = prismTools[args.tool];
    const siteId = await runSite(session, runSeq);
    if (handler === undefined || siteId === null) {
      result = toolFailure('error', ERROR_MESSAGE);
    } else {
      const base = { userId: session.userId, session, siteId, toolCallId: args.toolCallId, agent, afterCommit: undefined };
      try {
        result =
          TOOL_META[args.tool]?.tier === 'read'
            ? await handler({ ...base, executor: db }, args.input)
            : await withToolLedger(session, call, (tx, afterCommit) => handler({ ...base, executor: tx, afterCommit }, args.input));
      } catch (err) {
        log.warn('prism tool handler failed: {tool} {*}', { tool: args.tool, error: err });
        result = toolFailure('error', ERROR_MESSAGE);
      }
    }
  }

  await prism.resolveTool(args.agentId, args.toolCallId, result);
};
