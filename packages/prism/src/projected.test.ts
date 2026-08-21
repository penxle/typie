import { describe, expect, it } from 'vitest';
import { ProjectedEventSchema, ProjectedWorkflowEventSchema } from './projected.ts';

describe('run.started command', () => {
  it('command를 보존하고, 없거나 null이면 null로 정규화한다', () => {
    const expanded = '<command name="리뷰">\n본문\n</command>';
    expect(
      ProjectedEventSchema.parse({
        kind: 'run.started',
        data: { message: expanded, key: 'k', tag: null, command: { name: '리뷰', args: '' } },
      }).data,
    ).toEqual({ message: expanded, command: { name: '리뷰', args: '' } });
    expect(ProjectedEventSchema.parse({ kind: 'run.started', data: { message: '안녕', command: null } }).data).toEqual({
      message: '안녕',
      command: null,
    });
    expect(ProjectedEventSchema.parse({ kind: 'run.started', data: { message: '안녕' } }).data).toEqual({ message: '안녕', command: null });
  });
});

describe('tool data registry', () => {
  it('등록 도구는 data를 스키마로 좁히고, 미등재 도구는 data를 떨군다', () => {
    expect(
      ProjectedEventSchema.parse({
        kind: 'tool.requested',
        data: { tool: 'confirm-review', data: { documentId: 'D1', tier: 'high', extra: 1 } },
      }).data,
    ).toEqual({ tool: 'confirm-review', data: { documentId: 'D1', tier: 'high' } });
    expect(ProjectedEventSchema.parse({ kind: 'tool.requested', data: { tool: 'write', data: { path: 'x' } } }).data).toEqual({
      tool: 'write',
    });
    expect(ProjectedEventSchema.parse({ kind: 'tool.resolved', data: { tool: 'ask-user', ok: false, data: null } }).data).toEqual({
      tool: 'ask-user',
      ok: false,
    });
    expect(
      ProjectedEventSchema.parse({
        kind: 'tool.resolved',
        data: { tool: 'ask-user', ok: true, data: { answers: [{ question: 'Q', choice: ['a'] }] } },
      }).data,
    ).toEqual({ tool: 'ask-user', ok: true, data: { answers: [{ question: 'Q', choice: ['a'] }] } });
  });

  it('confirm-review 해소 결과는 결정 전문(key·tier·document)을 보존한다', () => {
    const confirmed = {
      decision: 'confirmed',
      key: 'r1',
      tier: 'medium',
      document: { title: 't', subtitle: null, path: 'manuscript/r1.md' },
    };
    expect(ProjectedEventSchema.parse({ kind: 'tool.resolved', data: { tool: 'confirm-review', ok: true, data: confirmed } }).data).toEqual(
      {
        tool: 'confirm-review',
        ok: true,
        data: confirmed,
      },
    );
    expect(
      ProjectedEventSchema.parse({ kind: 'tool.resolved', data: { tool: 'confirm-review', ok: true, data: { decision: 'declined' } } })
        .data,
    ).toEqual({ tool: 'confirm-review', ok: true, data: { decision: 'declined' } });
    expect(() =>
      ProjectedEventSchema.parse({ kind: 'tool.resolved', data: { tool: 'confirm-review', ok: true, data: { decision: 'confirmed' } } }),
    ).toThrow();
  });

  it('등록 도구의 data 형태 위반은 던진다', () => {
    expect(() =>
      ProjectedEventSchema.parse({ kind: 'tool.requested', data: { tool: 'confirm-review', data: { tier: 'ultra' } } }),
    ).toThrow();
  });

  it('형태 위반은 safeParse에서 던지지 않고 실패로 돌아온다', () => {
    const result = ProjectedEventSchema.safeParse({ kind: 'tool.requested', data: { tool: 'confirm-review', data: { tier: 'ultra' } } });
    expect(result.success).toBe(false);
    expect(result.error?.issues.map((issue) => issue.path)).toContainEqual(['data', 'data', 'tier']);
  });

  it('invocation.started는 workflow target의 id·name·app만 남긴다', () => {
    expect(
      ProjectedEventSchema.parse({
        kind: 'invocation.started',
        data: { ordinal: 0, target: { kind: 'workflow', id: 'w1', name: 'high', app: 'app_1', input: {}, files: [] } },
      }).data,
    ).toEqual({ target: { kind: 'workflow', id: 'w1', name: 'high', app: 'app_1' } });
  });

  it('agent.created는 프롬프트·도구 선언까지 통째로 벗기고 빈 data만 남긴다', () => {
    expect(
      ProjectedEventSchema.parse({
        kind: 'agent.created',
        data: {
          app: 'assistant',
          agent: 'main',
          prompt: 'p'.repeat(10_000),
          provider: 'anthropic',
          model: 'm',
          effort: null,
          mounts: {},
          tools: [{ kind: 'external', name: 'confirm-review', description: 'd', inputSchema: {}, resultSchema: {} }],
          files: [],
        },
      }).data,
    ).toEqual({});
  });

  it('워크플로·호출 종결은 result·usage를 통째로 벗기고 빈 data만 남긴다', () => {
    expect(
      ProjectedWorkflowEventSchema.parse({
        kind: 'workflow.completed',
        data: { result: '{"version":1,"kind":"issues","issues":[]}', usage: { settled: true, complete: true, folds: [] } },
      }).data,
    ).toEqual({});
    expect(
      ProjectedEventSchema.parse({
        kind: 'invocation.completed',
        data: { result: '{"version":1,"kind":"issues","issues":[]}', usage: { settled: true, complete: true, folds: [] } },
      }).data,
    ).toEqual({});
  });

  it('WORKFLOW 사영은 싣지 않는 kind를 거부한다', () => {
    expect(ProjectedWorkflowEventSchema.safeParse({ kind: 'agent.created', data: {} }).success).toBe(false);
    expect(ProjectedWorkflowEventSchema.safeParse({ kind: 'run.started', data: { message: 'x' } }).success).toBe(false);
    expect(ProjectedWorkflowEventSchema.safeParse({ kind: 'tool.rejected', data: { tool: 'write' } }).success).toBe(false);
  });
});

describe('workflow projection', () => {
  it('step·turn·tool.executed가 소비 필드만 남긴다', () => {
    expect(ProjectedWorkflowEventSchema.parse({ kind: 'step.started', data: {} }).data).toEqual({});
    expect(ProjectedWorkflowEventSchema.parse({ kind: 'step.completed', data: { result: { big: true } } }).data).toEqual({});
    expect(
      ProjectedWorkflowEventSchema.parse({
        kind: 'turn.completed',
        data: { text: '읽었어요', thinking: 't', toolCalls: [], stopReason: 'end', usage: null, raw: {} },
      }).data,
    ).toEqual({ text: '읽었어요' });
    expect(
      ProjectedWorkflowEventSchema.parse({
        kind: 'tool.executed',
        data: { tool: 'write', input: { path: 'notes/a.yaml', content: 'x'.repeat(5000) }, output: 'o', ok: true, data: null, duration: 3 },
      }).data,
    ).toEqual({ tool: 'write', ok: true, input: { path: 'notes/a.yaml' } });
    expect(
      ProjectedWorkflowEventSchema.parse({
        kind: 'tool.executed',
        data: { tool: 'websearch', input: { query: '회고 시점' }, output: 'o', ok: true, data: null, duration: 3 },
      }).data,
    ).toEqual({ tool: 'websearch', ok: true, input: { query: '회고 시점' } });
  });

  it('input이 객체가 아니거나 없으면 빈 input으로 떨어진다', () => {
    expect(
      ProjectedWorkflowEventSchema.parse({
        kind: 'tool.executed',
        data: { tool: 'list', input: 'x', output: '', ok: true, data: null, duration: 1 },
      }).data,
    ).toEqual({ tool: 'list', ok: true, input: {} });
    expect(
      ProjectedWorkflowEventSchema.parse({ kind: 'tool.executed', data: { tool: 'list', output: '', ok: true, data: null, duration: 1 } })
        .data,
    ).toEqual({ tool: 'list', ok: true, input: {} });
  });

  it('turn.completed의 text는 null을 허용하고 형태 위반은 던진다', () => {
    expect(ProjectedWorkflowEventSchema.parse({ kind: 'turn.completed', data: { text: null } }).data).toEqual({ text: null });
    expect(() => ProjectedWorkflowEventSchema.parse({ kind: 'turn.completed', data: { text: 1 } })).toThrow();
  });
});
