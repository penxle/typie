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
  // 반환한 콜백은 트랜잭션 커밋 뒤에 실행된다 — 커밋 전에 새어 나가면 안 되는 후처리(발행 등)를 싣는다
  onWorkflowSettled?(tx: Transaction, workflow: PrismWorkflowRow, outcome: WorkflowOutcome): Promise<(() => Promise<void>) | null>;
  onRunTerminal?(executor: Database | Transaction, sessionId: string, runSeq: number): Promise<void>;
};

export const prismApps: Record<string, PrismAppHooks> = { review: reviewHooks };
