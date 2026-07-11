export type SyncSession = { sessionId: string; userId: string; deviceId: string; bootstrapBypassKeyHash?: string };

export type DocumentAccess = 'ok' | 'forbidden' | 'not_v2';

export type BundleRow = { id: string; seq: number; payload: Uint8Array };

export type StreamEntry = { seq: string; changeset: Uint8Array };

export type ChangesetEvent = { target: string; seq: string; changesets: string[]; heads: string; durableHeads: string };

export type ChangesetSubscription = {
  [Symbol.asyncIterator]: () => AsyncIterator<ChangesetEvent>;
  return: () => unknown;
};

export type SyncDeps = {
  consumeTicket: (ticket: string) => Promise<SyncSession | null>;
  checkDocumentAccess: (userId: string, documentId: string) => Promise<DocumentAccess>;
  getCollectedSeq: (documentId: string) => Promise<string | null>;
  readBundleRow: (documentId: string, rowId: string) => Promise<BundleRow | null>;
  readBundlesAfter: (documentId: string, afterSeq: number, limit: number) => Promise<BundleRow[]>;
  readStreamBatch: (documentId: string, sinceSeq: string | null, count: number) => Promise<StreamEntry[]>;
  isStreamTruncated: (documentId: string, sinceSeq: string) => Promise<boolean>;
  hasStreamBeenTrimmed: (documentId: string) => Promise<boolean>;
  streamTip: (documentId: string) => Promise<string | null>;
  getLiveHeads: (documentId: string) => Promise<Uint8Array | null>;
  getDurableHeads: (documentId: string) => Promise<Uint8Array | null>;
  subscribeChangesets: (documentId: string) => ChangesetSubscription;
  peekOpsCount: (changesets: Uint8Array) => Promise<number>;
  appendBundle: (documentId: string, bundle: Uint8Array, userId: string, deviceId: string) => Promise<string>;
  advanceLiveHeads: (documentId: string, bundle: Uint8Array) => Promise<Uint8Array | null>;
  bootstrapLiveHeads: (documentId: string) => Promise<Uint8Array>;
  publishChangesets: (documentId: string, event: ChangesetEvent) => void;
  enqueueCollect: (documentId: string) => Promise<void>;
};
