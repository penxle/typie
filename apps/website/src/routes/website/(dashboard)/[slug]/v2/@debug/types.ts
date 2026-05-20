import type { PushStatus } from '../sync/types';

export type DebugSnapshot = {
  pushStatus: PushStatus;
  retryAttempt: number;
  lastSentHeadsBytes: number;
  hasEditor: boolean;
};

export type DebugEvent =
  | { kind: 'push.fired'; bytes: number }
  | { kind: 'push.success'; durationMs: number }
  | { kind: 'push.error'; message: string }
  | { kind: 'subscription.received'; bytes: number }
  | { kind: 'poll.applied'; bytes: number }
  | { kind: 'poll.error'; message: string };

export type TimelineEntry = { id: number; ts: number } & DebugEvent;

export type DebugEventCategory = 'push' | 'subscription' | 'poll';

export const eventCategory = (kind: DebugEvent['kind']): DebugEventCategory => {
  switch (kind) {
    case 'push.fired':
    case 'push.success':
    case 'push.error': {
      return 'push';
    }
    case 'subscription.received': {
      return 'subscription';
    }
    case 'poll.applied': {
      return 'poll';
    }
    case 'poll.error': {
      return 'poll';
    }
  }
};

export { type PushStatus } from '../sync/types';
