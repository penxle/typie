export type ClientCommitInput = {
  commitHash: string;
  parentCommitHash: string;
  rootObjectHash: string;
  steps: unknown;
  meta: unknown;
  committedAt: string;
};

export type DocumentObjectInput = {
  hash: string;
  content: unknown;
};

export type DebugSnapshot = {
  serverHeadHash: string;
  chainTip: string;
  localCommitChain: readonly ClientCommitInput[];
  pendingPushSet: ReadonlySet<string>;
  inflight: boolean;
  syncStatus: 'idle' | 'pushing' | 'error';
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
