export type PushStatus = 'idle' | 'pushing' | 'retrying' | 'error';

export type PusherEvent =
  | { kind: 'push.fired'; bytes: number }
  | { kind: 'push.success'; durationMs: number }
  | { kind: 'push.error'; message: string };
