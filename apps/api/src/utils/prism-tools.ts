import { reviewTools } from './prism-review.ts';
import type { AgentState } from '@typie/prism';
import type { PrismSessions } from '#/db/index.ts';

export type PrismToolContext = {
  userId: string;
  session: typeof PrismSessions.$inferSelect;
  toolCallId: string;
  agent: AgentState;
};
export type PrismToolHandler = (ctx: PrismToolContext, input: unknown) => Promise<unknown>;

export const prismTools: Record<string, PrismToolHandler> = { ...reviewTools };
