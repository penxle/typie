import type { DeltaStore } from './store';

export type SyncEditor = {
  currentHeads(): Uint8Array;
  changesetIds(): string[];
  missingChangesetsFor(confirmedHeads: Uint8Array): { bytes: Uint8Array; withheld: number };
  partitionRemoteChangesets(payload: Uint8Array): { ready: Uint8Array; blocked: Uint8Array };
  splitChangesets(payload: Uint8Array): { id: string; bytes: Uint8Array }[];
  receiveRemoteChangeset(payload: Uint8Array): void;
  flush(): void;
};

export type PushStatus = 'idle' | 'pushing' | 'retrying' | 'error';

export type PusherEvent =
  | { kind: 'push.fired'; bytes: number }
  | { kind: 'push.success'; durationMs: number }
  | { kind: 'push.error'; message: string }
  | { kind: 'persist.withheld'; count: number };

export type PushResult = { heads: Uint8Array; durableHeads: Uint8Array };

export type PusherOpts = {
  editor: SyncEditor;
  documentId: string;
  initialServerHeads: Uint8Array;
  initialDurableHeads: Uint8Array;
  store: DeltaStore;
  pushFn: (changesets: Uint8Array) => Promise<PushResult>;
  broadcast?: (changeset: Uint8Array) => void;
  onEvent?: (event: PusherEvent) => void;
};
