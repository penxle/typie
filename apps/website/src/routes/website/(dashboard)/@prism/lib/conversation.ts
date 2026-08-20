import { match } from 'ts-pattern';
import { applyDelta, sealTurn, startTurn } from './delta.ts';
import type { ProjectedEventFrame, ProjectedStreamFrame } from '@typie/prism';
import type { TurnLive } from './delta.ts';

export type RunState = 'idle' | 'running' | 'failed' | 'canceled';

export type TranscriptMessage =
  | { role: 'user'; key: string; text: string; at: number }
  | { role: 'assistant'; key: string; text: string | null; toolCalls: { id: string; name: string }[]; at: number }
  | { role: 'tool'; key: string; name: string; phase: 'executed' | 'rejected' | 'requested' | 'resolved'; ok: boolean | null; at: number };

export type Transcript = {
  messages: TranscriptMessage[];
  run: RunState;
  retrying: boolean;
  live: TurnLive | null;
  cursor: number;
};

export const emptyTranscript = (): Transcript => ({
  messages: [],
  run: 'idle',
  retrying: false,
  live: null,
  cursor: 0,
});

const toolMessage = (
  event: ProjectedEventFrame,
  name: string,
  phase: 'executed' | 'rejected' | 'requested' | 'resolved',
  ok: boolean | null,
): TranscriptMessage => ({
  role: 'tool',
  key: `e${event.seq}`,
  name,
  phase,
  ok,
  at: event.occurredAt,
});

const applyEvent = (t: Transcript, event: ProjectedEventFrame): Transcript => {
  if (event.seq <= t.cursor) return t;
  const next = { ...t, cursor: event.seq };
  return match(event)
    .with({ kind: 'run.started' }, ({ data }) => {
      const message: TranscriptMessage = { role: 'user', key: `e${event.seq}`, text: data.message, at: event.occurredAt };
      return { ...next, run: 'running' as const, retrying: false, messages: [...t.messages, message] };
    })
    .with({ kind: 'run.completed' }, () => ({ ...next, run: 'idle' as const, live: null, retrying: false }))
    .with({ kind: 'run.failed' }, () => ({ ...next, run: 'failed' as const, live: null, retrying: false }))
    .with({ kind: 'run.canceled' }, () => ({ ...next, run: 'canceled' as const, live: null, retrying: false }))
    .with({ kind: 'turn.started' }, () => ({ ...next, live: startTurn(t.live, event.context), retrying: false }))
    .with({ kind: 'turn.retried' }, () => ({ ...next, retrying: true, live: null }))
    .with({ kind: 'turn.completed' }, ({ data }) => {
      const toolCalls = data.toolCalls.map((call) => ({ id: call.id, name: call.name }));
      const sealed = { ...next, live: sealTurn(t.live, event.context), retrying: false };
      if (data.text === null && toolCalls.length === 0) return sealed;
      const message: TranscriptMessage = {
        role: 'assistant',
        key: `e${event.seq}`,
        text: data.text,
        toolCalls,
        at: event.occurredAt,
      };
      return { ...sealed, messages: [...t.messages, message] };
    })
    .with({ kind: 'tool.executed' }, ({ data }) => ({
      ...next,
      messages: [...t.messages, toolMessage(event, data.tool, 'executed', data.ok)],
    }))
    .with({ kind: 'tool.rejected' }, ({ data }) => ({
      ...next,
      messages: [...t.messages, toolMessage(event, data.tool, 'rejected', false)],
    }))
    .with({ kind: 'tool.requested' }, ({ data }) => ({
      ...next,
      messages: [...t.messages, toolMessage(event, data.tool, 'requested', null)],
    }))
    .with({ kind: 'tool.resolved' }, ({ data }) => ({
      ...next,
      messages: [...t.messages, toolMessage(event, data.tool, 'resolved', data.ok)],
    }))
    .exhaustive();
};

export const applyFrame = (t: Transcript, frame: ProjectedStreamFrame): Transcript =>
  match(frame)
    .with({ type: 'sync' }, ({ seq }) => ({ ...t, cursor: Math.max(t.cursor, seq) }))
    .with({ type: 'delta' }, ({ delta }) => ({ ...t, live: applyDelta(t.live, delta) }))
    .with({ type: 'event' }, ({ event }) => applyEvent(t, event))
    .exhaustive();
