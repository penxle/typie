import { logger } from '@typie/lib';
import dayjs from 'dayjs';
import { and, eq } from 'drizzle-orm';
import { db, first, firstOrThrow, PrismSessions, PrismWorkflows } from '#/db/index.ts';
import { activeRun, prism } from '#/external/prism.ts';
import { prismApps } from './prism-apps.ts';
import type { Database, Transaction } from '#/db/index.ts';
import type { PrismWorkflowRow } from './prism-apps.ts';

const log = logger.getChild('prism');

export type WorkflowDescriptor = {
  prismWorkflowId: string;
  app: string;
  name: string;
  ref: string | null;
  startedAt: number;
};

// invocation.started가 실어 온 서술 그대로 링크한다 — prism에 되묻지 않는 이유가 이 함수의 존재 이유다:
// 그 이벤트는 대상이 주소 가능해진 뒤 발행되지만, 로그 재생은 그 앞(할당 직후 죽음)도 본다.
export const linkWorkflowFromEvent = async (
  tx: Transaction,
  sessionId: string,
  descriptor: WorkflowDescriptor,
): Promise<PrismWorkflowRow> => {
  const existing = await tx.select().from(PrismWorkflows).where(eq(PrismWorkflows.prismWorkflowId, descriptor.prismWorkflowId)).then(first);
  if (existing) return existing;

  const row = await tx
    .insert(PrismWorkflows)
    .values({
      sessionId,
      prismWorkflowId: descriptor.prismWorkflowId,
      app: descriptor.app,
      name: descriptor.name,
      ref: descriptor.ref,
      startedAt: dayjs(descriptor.startedAt),
    })
    .onConflictDoNothing({ target: PrismWorkflows.prismWorkflowId })
    .returning()
    .then(first);
  if (!row)
    return tx.select().from(PrismWorkflows).where(eq(PrismWorkflows.prismWorkflowId, descriptor.prismWorkflowId)).then(firstOrThrow);

  await prismApps[row.app]?.onWorkflowLinked?.(tx, row);
  return row;
};

export const titleSession = async (executor: Database | Transaction, sessionId: string, title: string): Promise<void> => {
  await executor.update(PrismSessions).set({ title }).where(eq(PrismSessions.id, sessionId));
};

export const closeRun = async (executor: Database | Transaction, sessionId: string, runSeq: number): Promise<void> => {
  for (const hooks of Object.values(prismApps)) {
    await hooks.onRunTerminal?.(executor, sessionId, runSeq);
  }

  await executor
    .update(PrismSessions)
    .set({ openRunSeq: null })
    .where(and(eq(PrismSessions.id, sessionId), eq(PrismSessions.openRunSeq, runSeq)));
};

export const cancelActiveRun = async (prismAgentId: string): Promise<void> => {
  const agent = await prism.getAgent(prismAgentId);
  const running = activeRun(agent.runs);
  if (running) await prism.cancelAgentRun(prismAgentId, running.runSeq);
};

export const cancelSessionWorkflows = async (sessionId: string): Promise<void> => {
  const rows = await db
    .select({ prismWorkflowId: PrismWorkflows.prismWorkflowId })
    .from(PrismWorkflows)
    .where(and(eq(PrismWorkflows.sessionId, sessionId), eq(PrismWorkflows.state, 'RUNNING')));

  for (const row of rows) {
    await prism
      .cancelWorkflow(row.prismWorkflowId)
      .catch((err) => log.warn('workflow cancel failed {id}: {*}', { id: row.prismWorkflowId, error: err }));
  }
};
