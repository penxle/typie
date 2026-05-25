export type PushStatus = 'idle' | 'pushing' | 'retrying' | 'error';

export type PusherEvent =
  | { kind: 'push.fired'; bytes: number }
  | { kind: 'push.success'; durationMs: number }
  | { kind: 'push.error'; message: string };

export type OutboxEntry = {
  id: string;
  bundle: Uint8Array;
};

export type Outbox = {
  save(entry: OutboxEntry): Promise<void>;
  delete(id: string): Promise<void>;
  loadAll(): Promise<OutboxEntry[]>;
};
