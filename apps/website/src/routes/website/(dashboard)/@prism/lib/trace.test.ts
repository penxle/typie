import { describe, expect, it } from 'vitest';
import { applyTraceDelta, applyTraceEvent, currentStep, emptyTrace } from './trace.ts';
import type { ProjectedEventData, ProjectedEventFrame } from '@typie/prism';

const child = { id: 'agent_1', name: 'description-medium' };

const ev = (seq: number, kind: string, context: Record<string, unknown>, data: Record<string, unknown>): ProjectedEventFrame => ({
  seq,
  occurredAt: 1000 + seq,
  context: context as never,
  source: 'WORKFLOW',
  workflowId: 'w1',
  ...({ kind, data } as ProjectedEventData),
});

describe('applyTraceEvent', () => {
  it('step.started/completed가 스텝 목록을 열고 닫으며 currentStep은 열린 마지막 스텝이다', () => {
    let t = applyTraceEvent(emptyTrace(), ev(1, 'step.started', { step: 'manuscript' }, {}));
    expect(currentStep(t)).toBe('manuscript');
    t = applyTraceEvent(t, ev(2, 'step.completed', { step: 'manuscript' }, {}));
    expect(t.steps).toEqual([{ name: 'manuscript', seq: 1, startedAt: 1001, completedAt: 1002 }]);
    expect(currentStep(t)).toBe('manuscript');
    t = applyTraceEvent(t, ev(3, 'step.started', { step: 'description-0' }, {}));
    expect(currentStep(t)).toBe('description-0');
  });

  it('turn.completed는 text가 있을 때만 발화를 남기고 라이브 턴을 봉인한다', () => {
    let t = applyTraceEvent(emptyTrace(), ev(1, 'step.started', { step: 'description-0' }, {}));
    t = applyTraceDelta(t, {
      context: { agent: child, run: 1, turn: 1, attempt: 1 },
      channel: 'text',
      offset: 0,
      data: '첫',
      workflowId: 'w1',
    });
    expect(t.live?.text).toBe('첫');
    t = applyTraceEvent(
      t,
      ev(2, 'turn.completed', { agent: child, run: 1, turn: 1, attempt: 1, step: 'description-0' }, { text: '첫 구획' }),
    );
    expect(t.live).toBeNull();
    expect(t.turns).toEqual([{ seq: 2, step: 'description-0', text: '첫 구획', at: 1002 }]);
    t = applyTraceEvent(t, ev(3, 'turn.completed', { agent: child, run: 1, turn: 2, attempt: 1 }, { text: null }));
    expect(t.turns).toHaveLength(1);
  });

  it('turn/tool에 context.step이 없으면 현재 스텝에 귀속된다', () => {
    let t = applyTraceEvent(emptyTrace(), ev(1, 'step.started', { step: 'judgment-0' }, {}));
    t = applyTraceEvent(
      t,
      ev(
        2,
        'tool.executed',
        { agent: child, run: 1, turn: 1, attempt: 1, toolCallId: 'c' },
        { tool: 'read', ok: true, input: { path: 'manuscript/a.md' } },
      ),
    );
    expect(t.tools).toEqual([{ seq: 2, step: 'judgment-0', tool: 'read', ok: true, path: 'manuscript/a.md', query: null, at: 1002 }]);
  });

  it('tool.executed는 실패도 사실로 남기고 input이 없으면 null로 채운다', () => {
    const t = applyTraceEvent(emptyTrace(), ev(1, 'tool.executed', { step: 's' }, { tool: 'grep', ok: false, input: {} }));
    expect(t.tools[0]).toMatchObject({ tool: 'grep', ok: false, path: null, query: null });
  });

  it('workflow.started·tool.requested는 자취를 바꾸지 않는다', () => {
    const t = applyTraceEvent(emptyTrace(), ev(1, 'workflow.started', {}, {}));
    expect(t).toEqual(emptyTrace());
    const asked = applyTraceEvent(
      t,
      ev(2, 'tool.requested', { agent: child, toolCallId: 'q' }, { tool: 'ask-user', data: { questions: [] } }),
    );
    expect(asked).toEqual(emptyTrace());
  });
});
