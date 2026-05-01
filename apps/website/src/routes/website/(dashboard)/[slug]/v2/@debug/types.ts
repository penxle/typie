import type { ObjectContent } from '@typie/editor-ffi/browser';
import type { OutboxEntry, PushStatus } from '../sync/types';

export type { ClientCommitInput, DocumentObjectInput, OutboxEntry, PushStatus } from '../sync/types';

export type DebugSnapshot = {
  serverHeadHash: string;
  chainTip: string;
  outbox: readonly OutboxEntry[];
  cacheObjects: ReadonlyMap<string, ObjectContent>;
  pushStatus: PushStatus;
  retryAttempt: number;
  hasDocState: boolean;
};

export type DebugEvent =
  | { kind: 'commit.created'; hash: string; chainSize: number }
  | { kind: 'push.fired'; commits: number; objects: number }
  | { kind: 'push.success'; durationMs: number }
  | { kind: 'push.error'; message: string }
  | {
      kind: 'subscription.received';
      commits: number;
      objects: number;
      ownEcho: boolean;
      newHead: string;
    };

export type TimelineEntry = { id: number; ts: number } & DebugEvent;

export type DebugEventCategory = 'commit' | 'push' | 'subscription';

export const eventCategory = (kind: DebugEvent['kind']): DebugEventCategory => {
  switch (kind) {
    case 'commit.created': {
      return 'commit';
    }
    case 'push.fired':
    case 'push.success':
    case 'push.error': {
      return 'push';
    }
    case 'subscription.received': {
      return 'subscription';
    }
  }
};
