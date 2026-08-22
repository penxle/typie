import { logger } from '@typie/lib';
import { ASK_USER_TOOL, AskQuestionsSchema } from '@typie/prism';
import { and, eq, isNotNull, isNull } from 'drizzle-orm';
import { db, PrismSessions, PrismWorkflows } from '#/db/index.ts';
import { prism, PrismApiError } from '#/external/prism.ts';
import { cancelActiveRun, closeRun, linkWorkflow, settleWorkflow } from '#/utils/prism-workflows.ts';
import { isRunRunning, settleUpdate, workflowTargets } from '#/utils/prism-workflows-core.ts';
import { defineCron } from '../types.ts';
import { askBody, pushKey, subjectTitle } from './prism-core.ts';
import type { InvocationSummary } from '@typie/prism';
import type { PrismWorkflowRow } from '#/utils/prism-apps.ts';

const log = logger.getChild('prism-cron');

type PollRow = { workflow: PrismWorkflowRow; userId: string; sessionTitle: string | null };

const discover = async () => {
  const sessions = await db
    .select({ id: PrismSessions.id, prismAgentId: PrismSessions.prismAgentId, openRunSeq: PrismSessions.openRunSeq })
    .from(PrismSessions)
    .where(and(isNotNull(PrismSessions.openRunSeq), isNull(PrismSessions.deletedAt)));

  for (const session of sessions) {
    const openRunSeq = session.openRunSeq;
    if (openRunSeq === null) continue;

    try {
      const agent = await prism.getAgent(session.prismAgentId);

      const observed = await db
        .select({ prismWorkflowId: PrismWorkflows.prismWorkflowId })
        .from(PrismWorkflows)
        .where(eq(PrismWorkflows.sessionId, session.id));
      const known = new Set(observed.map((row) => row.prismWorkflowId));

      for (const workflowId of workflowTargets(agent.invocations)) {
        if (known.has(workflowId)) continue;
        try {
          await linkWorkflow(session.id, workflowId);
        } catch (err) {
          log.warn('link failed for workflow {id}: {*}', { id: workflowId, error: err });
        }
      }

      if (!isRunRunning(agent.runs, openRunSeq)) await closeRun(session.id, openRunSeq);
    } catch (err) {
      if (err instanceof PrismApiError && err.status === 404) {
        await closeRun(session.id, openRunSeq);
        continue;
      }

      log.warn('discover failed for session {id}: {*}', { id: session.id, error: err });
    }
  }
};

const pushQuestions = async (row: PollRow, invocations: InvocationSummary[]) => {
  for (const invocation of invocations) {
    if (invocation.targetKind !== 'agent' || invocation.status !== 'running') continue;

    const child = await prism.getAgent(invocation.targetId);
    if (child.pending?.tool !== ASK_USER_TOOL) continue;

    const parsed = AskQuestionsSchema.safeParse(child.pending.data);
    if (!parsed.success) continue;

    const { sendPushNotificationOnce } = await import('#/external/firebase.ts');
    await sendPushNotificationOnce({
      key: pushKey.ask(child.pending.toolCallId),
      userId: row.userId,
      title: `질문이 있어요 — ${subjectTitle(row.sessionTitle)}`,
      body: askBody(parsed.data.questions),
    });
  }
};

const poll = async () => {
  const rows = await db
    .select({ workflow: PrismWorkflows, userId: PrismSessions.userId, sessionTitle: PrismSessions.title })
    .from(PrismWorkflows)
    .innerJoin(PrismSessions, eq(PrismSessions.id, PrismWorkflows.sessionId))
    .where(and(eq(PrismWorkflows.state, 'RUNNING'), isNull(PrismSessions.deletedAt)));

  for (const row of rows) {
    try {
      const state = await prism.getWorkflow(row.workflow.prismWorkflowId);

      if (settleUpdate(state.workflow) === null) {
        await pushQuestions(row, state.invocations);
        continue;
      }

      await settleWorkflow(row.workflow, state);
    } catch (err) {
      log.warn('poll failed for workflow {id}: {*}', { id: row.workflow.prismWorkflowId, error: err });
    }
  }
};

const closeDeleted = async () => {
  const sessions = await db
    .select({ id: PrismSessions.id, prismAgentId: PrismSessions.prismAgentId, openRunSeq: PrismSessions.openRunSeq })
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

      await closeRun(session.id, openRunSeq);
    } catch (err) {
      log.warn('deleted-session close failed for session {id}: {*}', { id: session.id, error: err });
    }
  }
};

export const PrismWorkflowsPollCron = defineCron('prism:workflows:poll', '* * * * *', async () => {
  await discover();
  await poll();
  await closeDeleted();
});
