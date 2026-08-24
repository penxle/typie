// 순수 — env·DB·네트워크 import 없음(node:test 직접 로드)
import { applyFrame, emptyTranscript } from '@typie/prism';
import { projectFrame } from './prism-events.ts';
import type { Context, Transcript } from '@typie/prism';

export type StoredEvent = {
  seq: number;
  kind: string;
  occurredAt: number;
  loggedAt: number;
  context: Context | null;
  data: Record<string, unknown>;
};

const workflowIdOf = (event: StoredEvent): string | null => {
  if (event.kind !== 'invocation.started' || event.context === null) return null;
  const target = event.data.target as { kind?: unknown; id?: unknown } | undefined;
  return target?.kind === 'workflow' && typeof target.id === 'string' ? target.id : null;
};

export const materialize = (session: StoredEvent[], workflows: Map<string, StoredEvent[]>): Transcript => {
  let t = emptyTranscript();
  const apply = (event: StoredEvent, scope: Parameters<typeof projectFrame>[1]) => {
    if (event.context === null) return;
    const frame = projectFrame({ type: 'event', event }, scope);
    if (frame !== null) t = applyFrame(t, frame);
  };

  for (const event of session) {
    apply(event, { source: 'SESSION' });
    const workflowId = workflowIdOf(event);
    if (workflowId === null) continue;
    for (const child of workflows.get(workflowId) ?? []) apply(child, { source: 'WORKFLOW', workflowId });
  }

  return t;
};
