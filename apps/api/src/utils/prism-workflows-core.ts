// 순수 — env·DB·네트워크 import 없음(node:test 직접 로드)
import dayjs from 'dayjs';
import type { PrismWorkflowState } from '@typie/lib/enums';
import type { InvocationSummary, RunSummary, RunUsage, WorkflowState } from '@typie/prism';
import type { Dayjs } from 'dayjs';

const TERMINAL: Record<string, PrismWorkflowState> = { completed: 'COMPLETED', failed: 'FAILED', canceled: 'CANCELED' };

export const settleUpdate = (
  workflow: WorkflowState['workflow'],
): { state: PrismWorkflowState; usage: RunUsage | null; error: string | null; finishedAt: Dayjs } | null => {
  const state = TERMINAL[workflow.status];
  if (state === undefined) return null;

  return {
    state,
    usage:
      workflow.usage === null ? null : { complete: workflow.usage.settled ? workflow.usage.complete : false, folds: workflow.usage.folds },
    error: workflow.error,
    finishedAt: workflow.finishedAt === null ? dayjs() : dayjs(workflow.finishedAt),
  };
};

export const workflowTargets = (invocations: InvocationSummary[]): string[] =>
  invocations.filter((invocation) => invocation.targetKind === 'workflow').map((invocation) => invocation.targetId);

export const isRunningChildAgent = (invocations: InvocationSummary[], agentId: string): boolean =>
  invocations.some((invocation) => invocation.targetKind === 'agent' && invocation.targetId === agentId && invocation.status === 'running');

export const isRunRunning = (runs: RunSummary[], runSeq: number): boolean =>
  runs.some((run) => run.runSeq === runSeq && run.status === 'running');
