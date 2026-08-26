import { reviewTools } from './prism-review.ts';
import { workspaceTools } from './prism-workspace.ts';
import type { AgentState } from '@typie/prism';
import type { Database, PrismSessions, Transaction } from '#/db/index.ts';
import type { PostCommitRegistrar } from './post-commit.ts';

export type PrismToolContext = {
  userId: string;
  session: typeof PrismSessions.$inferSelect;
  siteId: string;
  toolCallId: string;
  agent: AgentState;
  executor: Database | Transaction;
  afterCommit: PostCommitRegistrar | undefined;
};
export type PrismToolHandler = (ctx: PrismToolContext, input: unknown) => Promise<unknown>;

export const prismTools: Record<string, PrismToolHandler> = { ...reviewTools, ...workspaceTools };
