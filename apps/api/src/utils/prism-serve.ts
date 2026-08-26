import { logger } from '@typie/lib';
import { PrismToolResolver, SiteState } from '@typie/lib/enums';
import { serveVerdict, TOOL_META, toolFailure } from '@typie/prism';
import { and, asc, eq } from 'drizzle-orm';
import { db, first, PrismRuns, PrismSessions, Sites } from '#/db/index.ts';
import { prism, PrismApiError } from '#/external/prism.ts';
import { recordToolResolution, withToolLedger } from './prism-tool-calls.ts';
import { DENIED_MESSAGE, ERROR_MESSAGE } from './prism-tool-messages.ts';
import { prismTools } from './prism-tools.ts';

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

  const call = { toolCallId: args.toolCallId, tool: args.tool, resolver: PrismToolResolver.SERVER };
  let result: unknown;
  if (verdict === 'deny') {
    result = toolFailure('denied', DENIED_MESSAGE);
    await recordToolResolution(session, call, result);
  } else {
    const handler = prismTools[args.tool];
    const siteId = await runSite(session, args.runSeq);
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
