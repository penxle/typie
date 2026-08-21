import { describe, expect, it } from 'vitest';
import { projectFrame } from './frames.ts';

const AGENT = { id: 'agent_a', name: 'description-high' };
const TURN = { agent: AGENT, run: 1, turn: 2, attempt: 1 };
const envelope = (kind: string, context: object, data: object, over: object = {}) => ({
  seq: 42,
  kind,
  occurredAt: 1000,
  loggedAt: 1003,
  context,
  data,
  ...over,
});

describe('projectFrame', () => {
  it('봉투의 좌표·시각은 그대로, loggedAt은 떨군다', () => {
    const frame = projectFrame(envelope('step.started', { step: 'description-0' }, {}));
    expect(frame).toEqual({ seq: 42, kind: 'step.started', occurredAt: 1000, context: { step: 'description-0' }, data: {} });
  });

  it('좌표는 계약 키만 옮기고 형태가 어긋난 값은 버린다', () => {
    const frame = projectFrame(envelope('turn.started', { ...TURN, invocation: 'inv_1', toolCallId: 7, turn: '2' }, {}));
    expect(frame?.context).toEqual({ agent: AGENT, run: 1, attempt: 1 });
  });

  it('turn.completed는 text·usage만 남긴다 — thinking·raw·toolCalls 전문은 떨어진다', () => {
    const usage = { inputTokens: 10, outputTokens: 5, cacheReadTokens: 1, cacheWriteTokens: 0, thinkingTokens: null };
    const data = {
      text: '말',
      thinking: 'x'.repeat(10_000),
      toolCalls: [{ kind: 'parsed', id: 'c', name: 'read', input: {} }],
      stopReason: 'end',
      usage,
      raw: { big: true },
    };
    expect(projectFrame(envelope('turn.completed', TURN, data))?.data).toEqual({ text: '말', usage });
    expect(projectFrame(envelope('turn.completed', TURN, { text: null, usage: null }))?.data).toEqual({ text: null, usage: null });
  });

  it('tool.executed는 도구·성패와 경로·질의·write 자수만 남긴다 — output·content 원문은 떨어진다', () => {
    const context = { ...TURN, toolCallId: 'call_1' };
    const write = envelope('tool.executed', context, {
      tool: 'write',
      input: { path: 'artifacts/a.yaml', content: '가'.repeat(3819) },
      output: 'ok',
      ok: true,
      data: null,
      duration: 3,
    });
    expect(projectFrame(write)?.data).toEqual({ tool: 'write', ok: true, input: { path: 'artifacts/a.yaml', chars: 3819 } });

    const read = envelope('tool.executed', context, {
      tool: 'read',
      input: { path: 'manuscript/v1.txt', offset: 0 },
      output: '원고 전문…',
      ok: true,
      data: {},
      duration: 1,
    });
    expect(projectFrame(read)?.data).toEqual({ tool: 'read', ok: true, input: { path: 'manuscript/v1.txt' } });

    const search = envelope('tool.executed', context, {
      tool: 'websearch',
      input: { query: '파쿠르' },
      output: '…',
      ok: false,
      data: null,
      duration: 9,
    });
    expect(projectFrame(search)?.data).toEqual({ tool: 'websearch', ok: false, input: { query: '파쿠르' } });
  });

  it('ask-user의 요청·해소 페이로드는 남기고, 다른 도구의 것은 떨군다', () => {
    const context = { ...TURN, toolCallId: 'call_1' };
    const questions = { questions: [{ question: 'Q?', hint: 'h', multi: false, options: [{ label: '가' }] }] };
    const requested = envelope('tool.requested', context, {
      tool: 'ask-user',
      input: { path: 'scratch/q.yaml' },
      data: questions,
      resultSchema: {},
    });
    expect(projectFrame(requested)?.data).toEqual({ tool: 'ask-user', data: questions });

    const answers = { answers: [{ question: 'Q?', choice: ['가'] }] };
    const resolved = envelope('tool.resolved', context, {
      tool: 'ask-user',
      input: {},
      output: 'Q?\n→ 가',
      ok: true,
      data: answers,
      duration: 2,
    });
    expect(projectFrame(resolved)?.data).toEqual({ tool: 'ask-user', ok: true, data: answers });

    const other = envelope('tool.requested', context, { tool: 'other-external', input: {}, data: { secret: 1 }, resultSchema: {} });
    expect(projectFrame(other)?.data).toEqual({ tool: 'other-external' });
  });

  it('생애주기·스텝 사실은 data를 비운다 — 종결 result·usage는 get 뷰가 정본이다', () => {
    expect(
      projectFrame(envelope('workflow.completed', {}, { result: { huge: true }, usage: { settled: true, complete: true, folds: [] } }))
        ?.data,
    ).toEqual({});
    expect(projectFrame(envelope('step.completed', { step: 'prepare' }, { result: { big: 'x' } }))?.data).toEqual({});
    expect(projectFrame(envelope('workflow.retried', {}, { count: 1, reissued: [] }))?.data).toEqual({});
  });

  it('사영 밖 kind는 null이다 — 자식 run·invocation·거절·고아·재시도·앱 이벤트', () => {
    for (const kind of [
      'run.started',
      'run.completed',
      'invocation.started',
      'tool.rejected',
      'tool.orphaned',
      'turn.retried',
      'agent.created',
      'feedback.custom',
    ]) {
      expect(projectFrame(envelope(kind, TURN, { any: 1 }))).toBeNull();
    }
  });

  it('구세대 행(context null)·깨진 봉투는 null이다', () => {
    expect(projectFrame(envelope('step.started', {}, {}, { context: null }))).toBeNull();
    expect(projectFrame(envelope('step.started', { step: 'x' }, {}, { seq: 'nope' }))).toBeNull();
    expect(projectFrame(null)).toBeNull();
    expect(projectFrame('frame')).toBeNull();
  });

  it('시각을 잃은 봉투는 occurredAt null로 통과한다', () => {
    expect(projectFrame(envelope('step.started', { step: 'x' }, {}, { occurredAt: undefined }))?.occurredAt).toBeNull();
  });
});
