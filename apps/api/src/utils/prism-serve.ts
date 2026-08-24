import { logger } from '@typie/lib';
import { SiteState } from '@typie/lib/enums';
import { serveVerdict, TOOL_META, toolFailure } from '@typie/prism';
import { and, asc, eq } from 'drizzle-orm';
import { db, first, firstOrThrow, PrismRuns, PrismSessions, PrismToolCalls, Sites } from '#/db/index.ts';
import { prism, PrismApiError } from '#/external/prism.ts';
import { DENIED_MESSAGE, ERROR_MESSAGE } from './prism-tool-messages.ts';
import { prismTools } from './prism-tools.ts';
import type { Transaction } from '#/db/index.ts';

const log = logger.getChild('prism-serve');

export const withToolLedger = async (
  session: { id: string },
  call: { toolCallId: string; tool: string },
  body: (tx: Transaction) => Promise<unknown>,
): Promise<unknown> =>
  await db.transaction(async (tx) => {
    const claimed = await tx
      .insert(PrismToolCalls)
      .values({ sessionId: session.id, toolCallId: call.toolCallId, tool: call.tool })
      .onConflictDoNothing({ target: [PrismToolCalls.sessionId, PrismToolCalls.toolCallId] })
      .returning({ id: PrismToolCalls.id })
      .then(first);

    if (!claimed) {
      const prior = await tx
        .select({ result: PrismToolCalls.result })
        .from(PrismToolCalls)
        .where(and(eq(PrismToolCalls.sessionId, session.id), eq(PrismToolCalls.toolCallId, call.toolCallId)))
        .then(firstOrThrow);
      return prior.result;
    }

    const result = await body(tx);
    await tx.update(PrismToolCalls).set({ result }).where(eq(PrismToolCalls.id, claimed.id));
    return result;
  });

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

export const serveTool = async (args: {
  sessionId: string;
  agentId: string;
  runSeq: number | null;
  toolCallId: string;
  tool: string;
  input: unknown;
}): Promise<void> => {
  const session = await db.select().from(PrismSessions).where(eq(PrismSessions.id, args.sessionId)).then(first);
  if (!session || session.deletedAt !== null) return;

  const verdict = serveVerdict(args.tool, session.toolPolicy);
  if (verdict === null) return;

  let agent;
  try {
    agent = await prism.getAgent(args.agentId);
  } catch (err) {
    if (err instanceof PrismApiError && err.status === 404) return;
    throw err;
  }
  if (!agent.pending || agent.pending.toolCallId !== args.toolCallId) return;

  let result: unknown;
  if (verdict === 'deny') {
    result = toolFailure('denied', DENIED_MESSAGE);
  } else {
    const handler = prismTools[args.tool];
    const siteId = await runSite(session, args.runSeq);
    if (handler === undefined || siteId === null) {
      result = toolFailure('error', ERROR_MESSAGE);
    } else {
      const base = { userId: session.userId, session, siteId, toolCallId: args.toolCallId, agent };
      try {
        result =
          TOOL_META[args.tool]?.tier === 'read'
            ? await handler({ ...base, executor: db }, args.input)
            : await withToolLedger(session, { toolCallId: args.toolCallId, tool: args.tool }, (tx) =>
                handler({ ...base, executor: tx }, args.input),
              );
      } catch (err) {
        log.warn('prism tool handler failed: {tool} {*}', { tool: args.tool, error: err });
        result = toolFailure('error', ERROR_MESSAGE);
      }
    }
  }

  await prism.resolveTool(args.agentId, args.toolCallId, result);
};
