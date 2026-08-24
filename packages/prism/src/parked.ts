import { toolResolver } from './tools.ts';
import type { Context } from './wire.ts';

export type ParkedEvent = {
  kind: string;
  context: Context | null;
  data: Record<string, unknown>;
};
export type ParkedScope = 'agent' | 'workflow';
export type ParkedOptions = { settledWorkflows?: ReadonlySet<string> };

const RUN_TERMINAL = new Set(['run.completed', 'run.failed', 'run.canceled']);
const INVOCATION_TERMINAL = new Set(['invocation.completed', 'invocation.failed', 'invocation.canceled']);

const runKey = (context: NonNullable<ParkedEvent['context']>): string => `${context.agent?.id ?? ''}#${context.run ?? 0}`;

export const parked = (events: ParkedEvent[], scope: ParkedScope, options: ParkedOptions = {}): boolean => {
  const openRuns = new Set<string>();
  const requests = new Map<string, { run: string; tool: string }>();
  const invocations = new Map<string, string>();

  for (const event of events) {
    const context = event.context;
    if (context === null) continue;
    const run = runKey(context);

    if (event.kind === 'run.started') {
      openRuns.add(run);
    } else if (RUN_TERMINAL.has(event.kind)) {
      openRuns.delete(run);
      for (const [id, request] of requests) if (request.run === run) requests.delete(id);
      for (const [id, owner] of invocations) if (owner === run) invocations.delete(id);
    } else if (event.kind === 'tool.requested' && context.toolCallId !== undefined) {
      requests.set(context.toolCallId, { run, tool: String(event.data.tool) });
    } else if (event.kind === 'tool.resolved' && context.toolCallId !== undefined) {
      requests.delete(context.toolCallId);
    } else if (scope === 'agent' && event.kind === 'invocation.started' && context.invocation !== undefined) {
      const target = event.data.target as { kind?: string; id?: string } | undefined;
      const settled = target?.id !== undefined && (options.settledWorkflows?.has(target.id) ?? false);
      if (!settled && target?.kind === 'workflow') invocations.set(context.invocation, run);
    } else if (scope === 'agent' && INVOCATION_TERMINAL.has(event.kind) && context.invocation !== undefined) {
      invocations.delete(context.invocation);
    }
  }

  const waiting = (run: string): boolean =>
    requests.values().some((request) => request.run === run && toolResolver(request.tool) === 'user');
  const invoking = (run: string): boolean => [...invocations.values()].includes(run);

  if (scope === 'agent') return [...openRuns].some((run) => waiting(run) || invoking(run));
  return openRuns.size > 0 && [...openRuns].every((run) => waiting(run));
};
