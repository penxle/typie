import { match } from 'ts-pattern';
import { applyDelta, sealTurn } from './delta.ts';
import type { TurnLive } from './delta.ts';
import type { ProjectedDeltaFrame, ProjectedEventFrame } from './projected.ts';

export type TranscriptStep = { name: string; seq: number; startedAt: number; completedAt: number | null };
export type TranscriptTurn = { seq: number; step: string | null; text: string; at: number };
export type TranscriptTool = {
  seq: number;
  step: string | null;
  tool: string;
  ok: boolean;
  path: string | null;
  query: string | null;
  at: number;
};

export type WorkflowTranscript = {
  steps: TranscriptStep[];
  turns: TranscriptTurn[];
  tools: TranscriptTool[];
  live: TurnLive | null;
};

export const emptyWorkflowTranscript = (): WorkflowTranscript => ({ steps: [], turns: [], tools: [], live: null });

export const currentStep = (transcript: WorkflowTranscript): string | null =>
  transcript.steps.findLast((step) => step.completedAt === null)?.name ?? transcript.steps.at(-1)?.name ?? null;

const stepOf = (transcript: WorkflowTranscript, event: ProjectedEventFrame): string | null => event.context.step ?? currentStep(transcript);

export const applyWorkflowTranscriptEvent = (transcript: WorkflowTranscript, event: ProjectedEventFrame): WorkflowTranscript =>
  match(event)
    .with({ kind: 'step.started' }, () => {
      const name = event.context.step;
      if (name === undefined) return transcript;
      return { ...transcript, steps: [...transcript.steps, { name, seq: event.seq, startedAt: event.occurredAt, completedAt: null }] };
    })
    .with({ kind: 'step.completed' }, () => ({
      ...transcript,
      steps: transcript.steps.map((step) =>
        step.name === event.context.step && step.completedAt === null ? { ...step, completedAt: event.occurredAt } : step,
      ),
    }))
    .with({ kind: 'turn.completed' }, ({ data }) => {
      const sealed = { ...transcript, live: sealTurn(transcript.live, event.context) };
      if (data.text === null) return sealed;
      return {
        ...sealed,
        turns: [...transcript.turns, { seq: event.seq, step: stepOf(transcript, event), text: data.text, at: event.occurredAt }],
      };
    })
    .with({ kind: 'tool.executed' }, ({ data }) => {
      const input = 'input' in data ? data.input : {};
      return {
        ...transcript,
        tools: [
          ...transcript.tools,
          {
            seq: event.seq,
            step: stepOf(transcript, event),
            tool: data.tool,
            ok: data.ok,
            path: input.path ?? null,
            query: input.query ?? null,
            at: event.occurredAt,
          },
        ],
      };
    })
    .otherwise(() => transcript);

export const applyWorkflowTranscriptDelta = (transcript: WorkflowTranscript, delta: ProjectedDeltaFrame): WorkflowTranscript => ({
  ...transcript,
  live: applyDelta(transcript.live, delta),
});
