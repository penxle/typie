import { toolResolver } from './tools.ts';
import type { ToolResolver } from './tools.ts';
import type { Context } from './wire.ts';

export type ParkedEvent = {
  kind: string;
  context: Context | null;
  data: Record<string, unknown>;
  occurredAt: number;
};
export type ParkedScope = 'agent' | 'workflow';
export type ParkedOptions = {
  settledWorkflows?: ReadonlySet<string>;
  resolverOf?: (tool: string) => ToolResolver;
  resolved?: ReadonlySet<string>;
};

const RUN_TERMINAL = new Set(['run.completed', 'run.failed', 'run.canceled']);
const INVOCATION_TERMINAL = new Set(['invocation.completed', 'invocation.failed', 'invocation.canceled']);

const runKey = (context: NonNullable<ParkedEvent['context']>): string => `${context.agent?.id ?? ''}#${context.run ?? 0}`;

type FoldedRequest = { run: string; runSeq: number | null; tool: string; input: unknown; agentId: string; at: number };
type Folded = { openRuns: Set<string>; requests: Map<string, FoldedRequest>; invocations: Map<string, string> };

const fold = (
  events: ParkedEvent[],
  scope: ParkedScope,
  settledWorkflows?: ReadonlySet<string>,
  resolved?: ReadonlySet<string>,
): Folded => {
  const openRuns = new Set<string>();
  const requests = new Map<string, FoldedRequest>();
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
      if (resolved?.has(context.toolCallId)) continue;
      requests.set(context.toolCallId, {
        run,
        runSeq: scope === 'agent' && typeof context.run === 'number' ? context.run : null,
        tool: String(event.data.tool),
        input: event.data.data,
        agentId: context.agent?.id ?? '',
        at: event.occurredAt,
      });
    } else if (event.kind === 'tool.resolved' && context.toolCallId !== undefined) {
      requests.delete(context.toolCallId);
    } else if (scope === 'agent' && event.kind === 'invocation.started' && context.invocation !== undefined) {
      const target = event.data.target as { kind?: string; id?: string } | undefined;
      const settled = target?.id !== undefined && (settledWorkflows?.has(target.id) ?? false);
      if (!settled && target?.kind === 'workflow') invocations.set(context.invocation, run);
    } else if (scope === 'agent' && INVOCATION_TERMINAL.has(event.kind) && context.invocation !== undefined) {
      invocations.delete(context.invocation);
    }
  }

  return { openRuns, requests, invocations };
};

export const parked = (events: ParkedEvent[], scope: ParkedScope, options: ParkedOptions = {}): boolean => {
  const resolverOf = options.resolverOf ?? toolResolver;
  const { openRuns, requests, invocations } = fold(events, scope, options.settledWorkflows, options.resolved);

  const waiting = (run: string): boolean => requests.values().some((request) => request.run === run && resolverOf(request.tool) === 'user');
  const invoking = (run: string): boolean => [...invocations.values()].includes(run);

  if (scope === 'agent') return [...openRuns].some((run) => waiting(run) || invoking(run));
  return openRuns.size > 0 && [...openRuns].every((run) => waiting(run));
};

export const awaitingUser = (
  events: ParkedEvent[],
  scope: ParkedScope,
  resolverOf: (tool: string) => ToolResolver = toolResolver,
): boolean => {
  const { openRuns, requests } = fold(events, scope);
  return [...openRuns].some((run) => [...requests.values()].some((request) => request.run === run && resolverOf(request.tool) === 'user'));
};

export type PendingRequest = { toolCallId: string; tool: string; input: unknown; agentId: string; runSeq: number | null; at: number };

export const pendingRequests = (events: ParkedEvent[], scope: ParkedScope): PendingRequest[] => {
  const { openRuns, requests } = fold(events, scope);
  return [...requests.entries()]
    .filter(([, request]) => openRuns.has(request.run))
    .map(([toolCallId, { tool, input, agentId, runSeq, at }]) => ({ toolCallId, tool, input, agentId, runSeq, at }));
};
