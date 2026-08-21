import { match } from 'ts-pattern';
import { applyDelta, sealTurn } from './delta.ts';
import type { ProjectedDeltaFrame, ProjectedEventFrame } from '@typie/prism';
import type { TurnLive } from './delta.ts';

export type TraceStep = { name: string; seq: number; startedAt: number; completedAt: number | null };
export type TraceTurn = { seq: number; step: string | null; text: string; at: number };
export type TraceTool = {
  seq: number;
  step: string | null;
  tool: string;
  ok: boolean;
  path: string | null;
  query: string | null;
  at: number;
};

export type WorkflowTrace = {
  steps: TraceStep[];
  turns: TraceTurn[];
  tools: TraceTool[];
  live: TurnLive | null;
};

export const emptyTrace = (): WorkflowTrace => ({ steps: [], turns: [], tools: [], live: null });

export const currentStep = (trace: WorkflowTrace): string | null =>
  trace.steps.findLast((step) => step.completedAt === null)?.name ?? trace.steps.at(-1)?.name ?? null;

const stepOf = (trace: WorkflowTrace, event: ProjectedEventFrame): string | null => event.context.step ?? currentStep(trace);

export const applyTraceEvent = (trace: WorkflowTrace, event: ProjectedEventFrame): WorkflowTrace =>
  match(event)
    .with({ kind: 'step.started' }, () => {
      const name = event.context.step;
      if (name === undefined) return trace;
      return { ...trace, steps: [...trace.steps, { name, seq: event.seq, startedAt: event.occurredAt, completedAt: null }] };
    })
    .with({ kind: 'step.completed' }, () => ({
      ...trace,
      steps: trace.steps.map((step) =>
        step.name === event.context.step && step.completedAt === null ? { ...step, completedAt: event.occurredAt } : step,
      ),
    }))
    .with({ kind: 'turn.completed' }, ({ data }) => {
      const sealed = { ...trace, live: sealTurn(trace.live, event.context) };
      if (data.text === null) return sealed;
      return { ...sealed, turns: [...trace.turns, { seq: event.seq, step: stepOf(trace, event), text: data.text, at: event.occurredAt }] };
    })
    .with({ kind: 'tool.executed' }, ({ data }) => {
      const input = 'input' in data ? data.input : {};
      return {
        ...trace,
        tools: [
          ...trace.tools,
          {
            seq: event.seq,
            step: stepOf(trace, event),
            tool: data.tool,
            ok: data.ok,
            path: input.path ?? null,
            query: input.query ?? null,
            at: event.occurredAt,
          },
        ],
      };
    })
    .otherwise(() => trace);

export const applyTraceDelta = (trace: WorkflowTrace, delta: ProjectedDeltaFrame): WorkflowTrace => ({
  ...trace,
  live: applyDelta(trace.live, delta),
});
