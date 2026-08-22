import { logger } from '@typie/lib';
import dayjs from 'dayjs';
import { and, eq } from 'drizzle-orm';
import { db, first, firstOrThrow, PrismSessions, PrismWorkflows } from '#/db/index.ts';
import { activeRun, prism } from '#/external/prism.ts';
import { prismApps } from './prism-apps.ts';
import { settleUpdate } from './prism-workflows-core.ts';
import type { WorkflowState } from '@typie/prism';
import type { PrismWorkflowRow } from './prism-apps.ts';

const log = logger.getChild('prism');

const selectByPrismId = (prismWorkflowId: string) =>
  db.select().from(PrismWorkflows).where(eq(PrismWorkflows.prismWorkflowId, prismWorkflowId));

export type WorkflowDescriptor = {
  prismWorkflowId: string;
  app: string;
  name: string;
  ref: string | null;
  startedAt: number;
};

const insertLink = async (sessionId: string, descriptor: WorkflowDescriptor): Promise<PrismWorkflowRow> => {
  const inserted = await db.transaction(async (tx) => {
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
    if (!row) return null;

    await prismApps[row.app]?.onWorkflowLinked?.(tx, row);
    return row;
  });

  if (!inserted) return selectByPrismId(descriptor.prismWorkflowId).then(firstOrThrow);

  return inserted;
};

// invocation.started가 실어 온 서술 그대로 링크한다 — prism에 되묻지 않는 이유가 이 함수의 존재 이유다:
// 그 이벤트는 대상이 주소 가능해진 뒤 발행되지만, 로그 재생은 그 앞(할당 직후 죽음)도 본다.
export const linkWorkflowFromEvent = async (sessionId: string, descriptor: WorkflowDescriptor): Promise<PrismWorkflowRow> => {
  const existing = await selectByPrismId(descriptor.prismWorkflowId).then(first);
  if (existing) return existing;

  return insertLink(sessionId, descriptor);
};

// 이벤트를 놓친 경로(크론 발견·워크플로 로그 자가 수리)의 링크 — 그 시점엔 워크플로가 이미 존재한다.
export const linkWorkflow = async (sessionId: string, prismWorkflowId: string): Promise<PrismWorkflowRow> => {
  const existing = await selectByPrismId(prismWorkflowId).then(first);
  if (existing) return existing;

  const { workflow } = await prism.getWorkflow(prismWorkflowId);
  const { app, workflow: name } = workflow;
  if (app === null || name === null) {
    throw new Error(`prism workflow ${prismWorkflowId} has no app or name`);
  }

  return insertLink(sessionId, { prismWorkflowId, app, name, ref: workflow.ref, startedAt: workflow.startedAt });
};

export const settleWorkflow = async (workflow: PrismWorkflowRow, state: WorkflowState): Promise<void> => {
  if (workflow.state !== 'RUNNING') return;

  const update = settleUpdate(state.workflow);
  if (update === null) return;

  await prismApps[workflow.app]?.onWorkflowSettled?.({ ...workflow, ...update }, state);

  await db
    .update(PrismWorkflows)
    .set(update)
    .where(and(eq(PrismWorkflows.id, workflow.id), eq(PrismWorkflows.state, 'RUNNING')));
};

export const titleSession = async (sessionId: string, title: string): Promise<void> => {
  await db.update(PrismSessions).set({ title }).where(eq(PrismSessions.id, sessionId));
};

export const closeRun = async (sessionId: string, runSeq: number): Promise<void> => {
  for (const hooks of Object.values(prismApps)) {
    await hooks.onRunTerminal?.(sessionId, runSeq);
  }

  await db
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
