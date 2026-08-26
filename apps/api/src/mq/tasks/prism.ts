import { logger } from '@typie/lib';
import { parked } from '@typie/prism';
import { and, eq, isNotNull, isNull } from 'drizzle-orm';
import { db, PrismSessions, PrismWorkflows } from '#/db/index.ts';
import { PrismApiError } from '#/external/prism.ts';
import { pubsub } from '#/pubsub.ts';
import { cancelActiveRun, closeRun } from '#/utils/prism-workflows.ts';
import { ensureIngest } from '../prism-queue.ts';
import { defineCron } from '../types.ts';
import { agentParked, loadParkedEvents } from './prism-ingest.ts';

const log = logger.getChild('prism-cron');

const sweepSessions = async () => {
  const sessions = await db
    .select({ id: PrismSessions.id })
    .from(PrismSessions)
    .where(and(isNotNull(PrismSessions.openRunSeq), isNull(PrismSessions.deletedAt)));

  for (const session of sessions) {
    try {
      const target = { kind: 'agent' as const, sessionId: session.id };
      if (await agentParked(session.id, await loadParkedEvents(target))) continue;
      await ensureIngest(target);
    } catch (err) {
      log.warn('session sweep failed for {id}: {*}', { id: session.id, error: err });
    }
  }
};

const sweepWorkflows = async () => {
  const workflows = await db
    .select({ id: PrismWorkflows.id })
    .from(PrismWorkflows)
    .innerJoin(PrismSessions, eq(PrismSessions.id, PrismWorkflows.sessionId))
    .where(and(eq(PrismWorkflows.state, 'RUNNING'), isNull(PrismSessions.deletedAt)));

  for (const workflow of workflows) {
    try {
      const target = { kind: 'workflow' as const, workflowId: workflow.id };
      if (parked(await loadParkedEvents(target), 'workflow')) continue;
      await ensureIngest(target);
    } catch (err) {
      log.warn('workflow sweep failed for {id}: {*}', { id: workflow.id, error: err });
    }
  }
};

const closeDeleted = async () => {
  const sessions = await db
    .select({
      id: PrismSessions.id,
      userId: PrismSessions.userId,
      prismAgentId: PrismSessions.prismAgentId,
      openRunSeq: PrismSessions.openRunSeq,
    })
    .from(PrismSessions)
    .where(and(isNotNull(PrismSessions.openRunSeq), isNotNull(PrismSessions.deletedAt)));

  for (const session of sessions) {
    const openRunSeq = session.openRunSeq;
    if (openRunSeq === null) continue;

    try {
      try {
        await cancelActiveRun(session.prismAgentId);
      } catch (err) {
        if (!(err instanceof PrismApiError && err.status === 404)) throw err;
      }

      await closeRun(db, session.id, openRunSeq);
      pubsub.publish('prism:credit', session.userId, {});
    } catch (err) {
      log.warn('deleted-session close failed for session {id}: {*}', { id: session.id, error: err });
    }
  }
};

export const PrismWorkflowsPollCron = defineCron('prism:workflows:poll', '* * * * *', async () => {
  await sweepSessions();
  await sweepWorkflows();
  await closeDeleted();
});
