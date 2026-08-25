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
      document: { id: 'D0DOC1', title: 't', subtitle: null, path: 'manuscript/r1.md' },
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

  it('delete-entities 요청은 ids를 보존하고, 형태 위반은 빈 목록으로 떨어진다', () => {
    expect(
      ProjectedEventSchema.parse({
        kind: 'tool.requested',
        data: { tool: 'delete-entities', data: { ids: ['E1', 'E2'], extra: 1 } },
      }).data,
    ).toEqual({ tool: 'delete-entities', data: { ids: ['E1', 'E2'] } });
    expect(ProjectedEventSchema.parse({ kind: 'tool.requested', data: { tool: 'delete-entities', data: { ids: 'E1' } } }).data).toEqual({
      tool: 'delete-entities',
      data: { ids: [] },
    });
    expect(ProjectedEventSchema.parse({ kind: 'tool.requested', data: { tool: 'delete-entities', data: null } }).data).toEqual({
      tool: 'delete-entities',
      data: { ids: [] },
    });
  });

  it('해소 주체(resolvedBy)는 있으면 그대로 지나가고 없으면 키 자체가 없다 — 카드/실행 줄 판정이 현재 정책이 아니라 이 값에 매인다', () => {
    const resolved = (data: Record<string, unknown>) => ProjectedEventSchema.parse({ kind: 'tool.resolved', data }).data;
    expect(resolved({ tool: 'delete-entities', ok: true, data: { ok: true, count: 1 }, resolvedBy: 'server' })).toEqual({
      tool: 'delete-entities',
      ok: true,
      resolvedBy: 'server',
      data: { ok: true, count: 1 },
    });
    expect(resolved({ tool: 'delete-entities', ok: true, data: { ok: true, count: 1 }, resolvedBy: 'user' })).toHaveProperty(
      'resolvedBy',
      'user',
    );
    expect(resolved({ tool: 'delete-entities', ok: true, data: { ok: true, count: 1 } })).not.toHaveProperty('resolvedBy');
    expect(() => resolved({ tool: 'delete-entities', ok: true, data: { ok: true, count: 1 }, resolvedBy: 'pump' })).toThrow();
  });

  it('delete-entities 해소 결과는 성공 봉투와 실패 봉투를 모두 보존한다', () => {
    expect(
      ProjectedEventSchema.parse({ kind: 'tool.resolved', data: { tool: 'delete-entities', ok: true, data: { ok: true, count: 2 } } }).data,
    ).toEqual({ tool: 'delete-entities', ok: true, data: { ok: true, count: 2 } });

    const declined = { ok: false, code: 'declined', message: '작가가 이 행동을 하지 않기로 했어요' };
    expect(ProjectedEventSchema.parse({ kind: 'tool.resolved', data: { tool: 'delete-entities', ok: true, data: declined } }).data).toEqual(
      { tool: 'delete-entities', ok: true, data: declined },
    );

    expect(() =>
      ProjectedEventSchema.parse({ kind: 'tool.resolved', data: { tool: 'delete-entities', ok: true, data: { ok: true } } }),
    ).toThrow();
  });

  it('되돌리기 어려운 나머지 셋(delete-notes·delete-goals·update-sharing)도 요청 data를 보존하고, 형태 위반은 빈 좌표로 떨어진다', () => {
    const requested = (tool: string, data: unknown) => ProjectedEventSchema.parse({ kind: 'tool.requested', data: { tool, data } }).data;
    expect(requested('delete-notes', { noteIds: ['N1', 'N2'], extra: 1 })).toEqual({
      tool: 'delete-notes',
      data: { noteIds: ['N1', 'N2'] },
    });
    expect(requested('delete-notes', { noteIds: [5] })).toEqual({ tool: 'delete-notes', data: { noteIds: [] } });
    expect(requested('delete-goals', { items: [{}] })).toEqual({ tool: 'delete-goals', data: { items: [{}] } });
    expect(requested('delete-goals', { items: [{ id: 'E1' }, {}] })).toEqual({ tool: 'delete-goals', data: { items: [{ id: 'E1' }, {}] } });
    expect(requested('delete-goals', { items: [{ id: 5 }] })).toEqual({ tool: 'delete-goals', data: { items: [] } });
    expect(requested('update-sharing', { ids: ['D1', 'E2'], visibility: 'PUBLIC', recursive: true, extra: 1 })).toEqual({
      tool: 'update-sharing',
      data: { ids: ['D1', 'E2'], visibility: 'PUBLIC', recursive: true },
    });
    expect(requested('update-sharing', { ids: ['E1'], visibility: 'SECRET' })).toEqual({
      tool: 'update-sharing',
      data: { ids: [], visibility: null },
    });
  });

  it('되돌리기 어려운 나머지 셋의 해소 결과는 성공 봉투와 실패 봉투를 모두 보존한다', () => {
    const resolved = (tool: string, data: unknown) =>
      ProjectedEventSchema.parse({ kind: 'tool.resolved', data: { tool, ok: true, data } }).data;
    const declined = { ok: false, code: 'declined', message: '작가가 이 행동을 하지 않기로 했어요' };
    expect(resolved('delete-notes', { ok: true, count: 2 })).toEqual({ tool: 'delete-notes', ok: true, data: { ok: true, count: 2 } });
    expect(resolved('delete-goals', { ok: true, count: 1 })).toEqual({ tool: 'delete-goals', ok: true, data: { ok: true, count: 1 } });
    const change = { id: 'E1', kind: 'document', title: '바다', from: 'PRIVATE', to: 'UNLISTED' };
    expect(resolved('update-sharing', { ok: true, count: 1, changes: [change] })).toEqual({
      tool: 'update-sharing',
      ok: true,
      data: { ok: true, count: 1, changes: [change] },
    });
    expect(() => resolved('update-sharing', { ok: true, count: 1 })).toThrow();
    for (const tool of ['delete-notes', 'delete-goals', 'update-sharing']) {
      expect(resolved(tool, declined)).toEqual({ tool, ok: true, data: declined });
    }
    expect(() => resolved('delete-notes', { ok: true })).toThrow();
    expect(() => resolved('delete-goals', { ok: true })).toThrow();
    expect(() => resolved('update-sharing', { ok: true })).toThrow();
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
