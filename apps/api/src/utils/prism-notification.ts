import type { PrismRunState } from '@typie/lib/enums';

export const PRISM_NOTIFICATION_KINDS = ['TURN_RESOLVED', 'USER_ACTION_REQUIRED'] as const;

export type PrismNotificationKind = (typeof PRISM_NOTIFICATION_KINDS)[number];
export type PrismNotificationPayload = {
  id: string;
  sessionId: string;
  kind: PrismNotificationKind;
  elapsedMs: number;
};

export const PRISM_NOTIFICATION_USER_ACTION_TTL_SECONDS = 24 * 60 * 60;
export const prismNotificationUserActionKey = (agentId: string): string => `prism:notification:user-action:${agentId}`;

const elapsed = (startedAt: number, finishedAt: number): number => Math.max(0, finishedAt - startedAt);
const responseStartedAt = (startedAt: number, lastUserActionAt: number | undefined, boundaryAt: number): number =>
  lastUserActionAt !== undefined && lastUserActionAt >= startedAt && lastUserActionAt <= boundaryAt ? lastUserActionAt : startedAt;

export const prismRunNotification = (args: {
  sessionId: string;
  runSeq: number;
  state: PrismRunState;
  startedAt: number;
  lastUserActionAt?: number;
  finishedAt: number;
}): PrismNotificationPayload | null => {
  if (args.state !== 'COMPLETED' && args.state !== 'FAILED') return null;

  return {
    id: `prism:notification:${args.sessionId}:run:${args.runSeq}:resolved`,
    sessionId: args.sessionId,
    kind: 'TURN_RESOLVED',
    elapsedMs: elapsed(responseStartedAt(args.startedAt, args.lastUserActionAt, args.finishedAt), args.finishedAt),
  };
};

export const prismUserActionNotification = (args: {
  sessionId: string;
  toolCallId: string;
  startedAt: number;
  lastUserActionAt?: number;
  requestedAt: number;
}): PrismNotificationPayload => ({
  id: `prism:notification:${args.sessionId}:tool:${args.toolCallId}:action-required`,
  sessionId: args.sessionId,
  kind: 'USER_ACTION_REQUIRED',
  elapsedMs: elapsed(responseStartedAt(args.startedAt, args.lastUserActionAt, args.requestedAt), args.requestedAt),
});
