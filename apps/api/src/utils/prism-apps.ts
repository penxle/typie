import { reviewHooks } from './prism-review.ts';
import type { PrismWorkflowState } from '@typie/lib/enums';
import type { RunUsage } from '@typie/prism';
import type { Dayjs } from 'dayjs';
import type { Database, PrismWorkflows, Transaction } from '#/db/index.ts';

export type PrismWorkflowRow = typeof PrismWorkflows.$inferSelect;

export type WorkflowOutcome = {
  state: PrismWorkflowState;
  result: unknown;
  usage: RunUsage | null;
  error: string | null;
  finishedAt: Dayjs;
};

export type PrismAppHooks = {
  onWorkflowLinked?(tx: Transaction, workflow: PrismWorkflowRow): Promise<void>;
  onWorkflowSettled?(tx: Transaction, workflow: PrismWorkflowRow, outcome: WorkflowOutcome): Promise<void>;
  onRunTerminal?(executor: Database | Transaction, sessionId: string, runSeq: number): Promise<void>;
};

export const prismApps: Record<string, PrismAppHooks> = { feedback: reviewHooks };
