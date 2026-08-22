import { reviewHooks } from './prism-review.ts';
import type { WorkflowState } from '@typie/prism';
import type { PrismWorkflows, Transaction } from '#/db/index.ts';

export type PrismWorkflowRow = typeof PrismWorkflows.$inferSelect;

export type PrismAppHooks = {
  onWorkflowLinked?(tx: Transaction, workflow: PrismWorkflowRow): Promise<void>;
  onWorkflowSettled?(workflow: PrismWorkflowRow, view: WorkflowState): Promise<void>;
  onRunTerminal?(sessionId: string, runSeq: number): Promise<void>;
  resolveSession?(ref: string | null): Promise<string | null>;
};

export const prismApps: Record<string, PrismAppHooks> = { feedback: reviewHooks };
