import { describe, expect, it } from 'vitest';
import { AgentStateSchema, StreamFrameSchema, WorkflowStateSchema } from './wire.ts';

describe('StreamFrameSchema', () => {
  it('프레임 4종을 판별한다', () => {
    expect(StreamFrameSchema.parse({ event: 'heartbeat', data: '{}' })).toEqual({ type: 'heartbeat' });
    expect(StreamFrameSchema.parse({ event: 'sync', data: '{"seq":9}' })).toEqual({ type: 'sync', seq: 9 });

    const delta = StreamFrameSchema.parse({
      event: 'turn.delta',
      data: JSON.stringify({
        context: { agent: { id: 'a', name: 'x' }, run: 1, turn: 1, attempt: 1 },
        channel: 'text',
        offset: 0,
        data: '안',
      }),
    });
    expect(delta).toEqual({
      type: 'delta',
      delta: { context: { agent: { id: 'a', name: 'x' }, run: 1, turn: 1, attempt: 1 }, channel: 'text', offset: 0, data: '안' },
    });

    const frame = StreamFrameSchema.parse({
      event: 'run.started',
      data: JSON.stringify({
        seq: 2,
        kind: 'run.started',
        occurredAt: 5,
        loggedAt: 5,
        context: { agent: { id: 'a', name: 'x' }, run: 1 },
        data: { message: '안녕' },
      }),
    });
    expect(frame).toEqual({
      type: 'event',
      event: { seq: 2, kind: 'run.started', occurredAt: 5, context: { agent: { id: 'a', name: 'x' }, run: 1 }, data: { message: '안녕' } },
    });
  });

  it('깨진 JSON·봉투 결손·tool 없는 tool.input 델타를 거부한다', () => {
    expect(StreamFrameSchema.safeParse({ event: 'run.started', data: '{broken' }).success).toBe(false);
    expect(StreamFrameSchema.safeParse({ event: 'run.started', data: '{"kind":"run.started"}' }).success).toBe(false);
    expect(
      StreamFrameSchema.safeParse({
        event: 'turn.delta',
        data: JSON.stringify({
          context: { agent: { id: 'a', name: 'x' }, run: 1, turn: 1, attempt: 1 },
          channel: 'tool.input',
          offset: 0,
          data: '{',
        }),
      }).success,
    ).toBe(false);
  });
});

const fold = {
  provider: 'anthropic',
  agent: 'chat',
  model: 'claude-opus-5',
  effort: null,
  turns: 3,
  inputTokens: 100,
  outputTokens: 20,
  cacheReadTokens: 4000,
  cacheWriteTokens: 0,
  thinkingTokens: 5,
};

describe('AgentStateSchema·WorkflowStateSchema', () => {
  it('AgentState.invocations는 요약 5열만 남기고 나머지는 벗긴다', () => {
    const parsed = AgentStateSchema.parse({
      runs: [],
      pending: null,
      invocations: [
        { invocationId: 'i1', targetKind: 'workflow', targetId: 'workflow_1', originRunSeq: 3, status: 'running', result: 'x' },
      ],
    });
    expect(parsed.invocations).toEqual([
      { invocationId: 'i1', targetKind: 'workflow', targetId: 'workflow_1', originRunSeq: 3, status: 'running' },
    ]);
  });

  it('WorkflowState는 app·이름·ref·status·result 문자열·usage 판별자를 읽는다', () => {
    const parsed = WorkflowStateSchema.parse({
      workflow: {
        id: 'workflow_1',
        app: 'app_1',
        workflow: 'high',
        ref: 'PRRR1',
        status: 'completed',
        result: '{"kind":"issues"}',
        error: null,
        usage: { settled: true, complete: true, folds: [fold] },
        startedAt: 1,
        finishedAt: 2,
        input: '{}',
      },
      invocations: [],
    });
    expect(parsed.workflow.app).toBe('app_1');
    expect(parsed.workflow.workflow).toBe('high');
    expect(parsed.workflow.ref).toBe('PRRR1');
    expect(parsed.workflow.usage).toEqual({ settled: true, complete: true, folds: [fold] });
  });

  it('usage fold의 열이 빠지면 파싱이 실패한다', () => {
    const broken = { ...fold, turns: undefined };
    const result = WorkflowStateSchema.safeParse({
      workflow: {
        id: 'workflow_1',
        app: 'app_1',
        workflow: 'high',
        ref: null,
        status: 'completed',
        result: null,
        error: null,
        usage: { settled: false, folds: [broken] },
        startedAt: 1,
        finishedAt: 2,
      },
      invocations: [],
    });
    expect(result.success).toBe(false);
  });

  it('WorkflowState의 app·이름은 굳기 전 워크플로를 위해 null을 받는다', () => {
    const parsed = WorkflowStateSchema.parse({
      workflow: {
        id: 'workflow_1',
        app: null,
        workflow: null,
        ref: null,
        status: 'running',
        result: null,
        error: null,
        usage: null,
        startedAt: 1,
        finishedAt: null,
      },
      invocations: [],
    });
    expect(parsed.workflow.app).toBeNull();
    expect(parsed.workflow.workflow).toBeNull();
  });
});
