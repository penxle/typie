import { logger } from '@typie/lib';
import { PrismToolResolver } from '@typie/lib/enums';
import { and, eq, inArray } from 'drizzle-orm';
import { db, first, firstOrThrow, PrismToolCalls } from '#/db/index.ts';
import { runPostCommitEffects } from './post-commit.ts';
import type { Transaction } from '#/db/index.ts';
import type { PostCommitEffect, PostCommitRegistrar } from './post-commit.ts';

const log = logger.getChild('prism-serve');

type ToolCallRef = { toolCallId: string; tool: string; resolver: PrismToolResolver };

export const withToolLedger = async (
  session: { id: string },
  call: ToolCallRef,
  body: (tx: Transaction, afterCommit: PostCommitRegistrar) => Promise<unknown>,
): Promise<unknown> => {
  const effects: PostCommitEffect[] = [];
  const result = await db.transaction(async (tx) => {
    const claimed = await tx
      .insert(PrismToolCalls)
      .values({ sessionId: session.id, toolCallId: call.toolCallId, tool: call.tool, resolver: call.resolver })
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

    const result = await body(tx, (effect) => {
      effects.push(effect);
    });
    await tx.update(PrismToolCalls).set({ result }).where(eq(PrismToolCalls.id, claimed.id));
    return result;
  });

  // The tool result and its idempotency ledger are already committed. Reporting a delivery failure as a tool failure
  // would make a retry return the cached success without retrying the effect, so run every effect and report failures separately.
  const errors = await runPostCommitEffects(effects);
  for (const error of errors) {
    log.error('prism post-commit effect failed: {tool} {*}', { tool: call.tool, toolCallId: call.toolCallId, error });
  }

  return result;
};

export const recordToolResolution = async (session: { id: string }, call: ToolCallRef, result: unknown): Promise<void> => {
  await db
    .insert(PrismToolCalls)
    .values({ sessionId: session.id, toolCallId: call.toolCallId, tool: call.tool, resolver: call.resolver, result })
    .onConflictDoNothing({ target: [PrismToolCalls.sessionId, PrismToolCalls.toolCallId] });
};

export const toolResolverOf = async (sessionId: string, toolCallId: string): Promise<PrismToolResolver | null> =>
  await db
    .select({ resolver: PrismToolCalls.resolver })
    .from(PrismToolCalls)
    .where(and(eq(PrismToolCalls.sessionId, sessionId), eq(PrismToolCalls.toolCallId, toolCallId)))
    .then(first)
    .then((row) => row?.resolver ?? null);

export const resolvedToolCallIds = async (sessionId: string, toolCallIds: string[]): Promise<Set<string>> => {
  if (toolCallIds.length === 0) return new Set();
  const rows = await db
    .select({ toolCallId: PrismToolCalls.toolCallId })
    .from(PrismToolCalls)
    .where(and(eq(PrismToolCalls.sessionId, sessionId), inArray(PrismToolCalls.toolCallId, toolCallIds)));
  return new Set(rows.map((row) => row.toolCallId));
};
