import { describe, expect, it } from 'vitest';
import { buildPassage, elapsedLabel, runningLabel, spentLabel, tailLabel, toolRowLabel } from './passage-view.ts';
import type { ToolRequestMessage, TranscriptTool, WorkflowTranscript } from '@typie/prism';

const tool = (seq: number, step: string, tool: string, extra: Partial<TranscriptTool> = {}): TranscriptTool => ({
  seq,
  step,
  tool,
  ok: true,
  path: null,
  query: null,
  at: 1000 + seq,
  ...extra,
});

const request = (seq: number, status: ToolRequestMessage['status'], settledAt?: number): ToolRequestMessage => ({
  role: 'tool-request',
  key: `w1:${seq}`,
  seq,
  tool: 'ask-user',
  toolCallId: `q${seq}`,
  agentId: 'a',
  workflowId: 'w1',
  data: { questions: [] },
  status,
  at: 1000 + seq,
  settledAt,
});

const transcript = (partial: Partial<WorkflowTranscript>): WorkflowTranscript => ({
  steps: [],
  turns: [],
  tools: [],
  live: null,
  ...partial,
});

describe('toolRowLabel', () => {
  it('작가의 말로 바꾸고 실패·list는 버린다', () => {
    expect(toolRowLabel(tool(1, 's', 'read', { path: 'manuscript/a.md' }))).toBe('원고를 읽었어요');
    expect(toolRowLabel(tool(1, 's', 'read', { path: 'notes/a.yaml' }))).toBe('메모를 읽었어요');
    expect(toolRowLabel(tool(1, 's', 'grep', { query: 'x.*y' }))).toBe('원고에서 찾아봤어요');
    expect(toolRowLabel(tool(1, 's', 'write'))).toBe('메모를 남겼어요');
    expect(toolRowLabel(tool(1, 's', 'edit'))).toBe('메모를 고쳤어요');
    expect(toolRowLabel(tool(1, 's', 'websearch', { query: '회고 시점' }))).toBe('웹에서 ‘회고 시점’을 찾아봤어요');
    expect(toolRowLabel(tool(1, 's', 'websearch'))).toBe('웹에서 찾아봤어요');
    expect(toolRowLabel(tool(1, 's', 'list'))).toBeNull();
    expect(toolRowLabel(tool(1, 's', 'read', { ok: false }))).toBeNull();
    expect(toolRowLabel(tool(1, 's', 'unknown'))).toBeNull();
  });
});

describe('tailLabel', () => {
  const live = {
    context: { agent: { id: 'c', name: 'x' }, run: 1, turn: 1, attempt: 1 },
    text: '',
    textBroken: false,
    thinkingChars: 0,
    toolInput: null,
    last: 'thinking' as const,
    seeded: false,
  };

  it('마지막 채널과 1:1 — 입력 문면은 write·edit뿐', () => {
    expect(tailLabel({ live: null, reconnecting: false, lateMs: 0 })).toBeNull();
    expect(tailLabel({ live: { ...live, thinkingChars: 3 }, reconnecting: false, lateMs: 0 })).toBe('생각하는 중');
    expect(tailLabel({ live: { ...live, last: 'tool.input', toolInput: { name: 'write' } }, reconnecting: false, lateMs: 0 })).toBe(
      '메모를 쓰는 중',
    );
    expect(tailLabel({ live: { ...live, last: 'tool.input', toolInput: { name: 'edit' } }, reconnecting: false, lateMs: 0 })).toBe(
      '메모를 고치는 중',
    );
    expect(tailLabel({ live: { ...live, last: 'tool.input', toolInput: { name: 'read' } }, reconnecting: false, lateMs: 0 })).toBeNull();
    expect(
      tailLabel({ live: { ...live, last: 'tool.input', toolInput: { name: 'ask-user' } }, reconnecting: false, lateMs: 0 }),
    ).toBeNull();
    expect(tailLabel({ live: { ...live, last: 'text', text: '흐르는 중' }, reconnecting: false, lateMs: 0 })).toBeNull();
  });

  it('쓰기가 끝나고 생각 델타가 오면 마지막 채널이 문면을 정한다', () => {
    expect(
      tailLabel({ live: { ...live, last: 'thinking', thinkingChars: 8, toolInput: { name: 'write' } }, reconnecting: false, lateMs: 0 }),
    ).toBe('생각하는 중');
  });

  it('늦어짐은 다른 문면을 덮고 재접속이 그보다 앞선다', () => {
    expect(tailLabel({ live: { ...live, thinkingChars: 3 }, reconnecting: false, lateMs: 31_000 })).toBe('응답이 늦어지고 있어요');
    expect(tailLabel({ live: null, reconnecting: true, lateMs: 31_000 })).toBe('다시 연결하는 중');
  });
});

describe('buildPassage', () => {
  const steps = [
    { name: 'manuscript', seq: 1, startedAt: 0, completedAt: 1000 },
    { name: 'classify-0', seq: 2, startedAt: 1000, completedAt: 20_000 },
    { name: 'description-0', seq: 3, startedAt: 20_000, completedAt: null },
  ];

  it('단계 상태·요약·경과·묶음', () => {
    const view = buildPassage({
      transcript: transcript({
        steps,
        turns: [
          { seq: 4, step: 'classify-0', text: '리뷰할 수 있는 단편이에요', at: 15_000 },
          { seq: 6, step: 'description-0', text: '첫 구획을 읽었어요', at: 60_000 },
          { seq: 9, step: 'description-0', text: '구획 지도를 그렸어요', at: 120_000 },
        ],
        tools: [
          tool(5, 'description-0', 'read', { path: 'manuscript/a.md', at: 30_000 }),
          tool(7, 'description-0', 'read', { path: 'manuscript/a.md', at: 70_000 }),
          tool(8, 'description-0', 'write', { at: 90_000 }),
          tool(10, 'description-0', 'list', { at: 130_000 }),
        ],
      }),
      status: 'running',
      tier: 'medium',
      requests: [],
      now: 200_000,
      finishedAt: null,
    });
    expect(view.current).toBe('description');
    expect(view.stages.map((s) => [s.key, s.status])).toEqual([
      ['classify', 'done'],
      ['description', 'running'],
      ['judgment', 'pending'],
      ['stylistic', 'pending'],
      ['delivery', 'pending'],
    ]);
    expect(view.stages[0]).toMatchObject({ summary: '리뷰할 수 있는 단편이에요', elapsedMs: 19_000 });
    expect(view.stages[1].elapsedMs).toBe(180_000);
    expect(view.stages.slice(2).map((s) => s.elapsedMs)).toEqual([null, null, null]);
    expect(view.stages[1].groups.map((g) => g.kind)).toEqual(['tools', 'narration', 'tools', 'narration']);
    expect(view.stages[1].groups[2]).toMatchObject({
      kind: 'tools',
      count: 2,
      rows: [
        { label: '원고를 읽었어요', count: 1 },
        { label: '메모를 남겼어요', count: 1 },
      ],
    });
    expect(view.elapsedMs).toBe(200_000);
  });

  it('연속 동일 행은 ×N으로 접힌다', () => {
    const view = buildPassage({
      transcript: transcript({
        steps,
        tools: [
          tool(5, 'description-0', 'read', { path: 'manuscript/a.md' }),
          tool(6, 'description-0', 'read', { path: 'manuscript/a.md' }),
          tool(7, 'description-0', 'write'),
        ],
      }),
      status: 'running',
      tier: 'medium',
      requests: [],
      now: 200_000,
      finishedAt: null,
    });
    expect(view.stages[1].groups[0]).toMatchObject({
      kind: 'tools',
      count: 3,
      rows: [
        { label: '원고를 읽었어요', count: 2 },
        { label: '메모를 남겼어요', count: 1 },
      ],
    });
  });

  it('질문은 seq 자리에 서고 대기 구간은 경과에서 빠진다', () => {
    const view = buildPassage({
      transcript: transcript({ steps, turns: [{ seq: 4, step: 'description-0', text: 'a', at: 30_000 }] }),
      status: 'running',
      tier: 'medium',
      requests: [request(5, 'resolved', 100_000)],
      now: 200_000,
      finishedAt: null,
    });
    expect(view.stages[1].groups.map((g) => g.kind)).toEqual(['narration', 'question']);
    expect(view.stages[1].elapsedMs).toBe(180_000 - (100_000 - 20_000));
  });

  it('아직 답하지 않은 질문은 지금까지가 통째로 대기다', () => {
    const view = buildPassage({
      transcript: transcript({ steps: [{ name: 'classify-0', seq: 1, startedAt: 0, completedAt: null }] }),
      status: 'running',
      tier: 'low',
      requests: [request(2, 'pending')],
      now: 10_000,
      finishedAt: null,
    });
    expect(view.elapsedMs).toBe(10_000 - (10_000 - 1002));
  });

  it('겹치는 대기 구간은 한 번만 깎는다', () => {
    const view = buildPassage({
      transcript: transcript({ steps: [{ name: 'classify-0', seq: 1, startedAt: 0, completedAt: null }] }),
      status: 'running',
      tier: 'low',
      requests: [request(2, 'resolved', 5000), request(3, 'resolved', 6000)],
      now: 10_000,
      finishedAt: null,
    });
    expect(view.elapsedMs).toBe(10_000 - (6000 - 1002));
  });

  it('requests는 워크플로 스코프여야 한다 — 루트 질문을 섞으면 경과가 잘못 깎인다', () => {
    const base = {
      transcript: transcript({ steps: [{ name: 'classify-0', seq: 1, startedAt: 0, completedAt: null }] }),
      status: 'running' as const,
      tier: 'low' as const,
      now: 10_000,
      finishedAt: null,
    };
    const scoped = buildPassage({ ...base, requests: [request(2, 'resolved', 3000)] });
    const mixed = buildPassage({
      ...base,
      requests: [request(2, 'resolved', 3000), { ...request(9, 'resolved', 9000), workflowId: undefined }],
    });
    expect(mixed.elapsedMs).toBeLessThan(scoped.elapsedMs);
  });

  it('점검 라운드는 소구역으로 묶인다', () => {
    const view = buildPassage({
      transcript: transcript({
        steps: [
          { name: 'rubric-0', seq: 1, startedAt: 0, completedAt: 1000 },
          { name: 'audit-1', seq: 2, startedAt: 1000, completedAt: 2000 },
          { name: 'calibration-1-0', seq: 3, startedAt: 2000, completedAt: 3000 },
          { name: 'rubric-revise-1-0', seq: 4, startedAt: 3000, completedAt: 4000 },
          { name: 'audit-2', seq: 5, startedAt: 4000, completedAt: null },
        ],
        turns: [
          { seq: 10, step: 'rubric-0', text: '초안', at: 500 },
          { seq: 11, step: 'calibration-1-0', text: '점검', at: 2500 },
          { seq: 12, step: 'rubric-revise-1-0', text: '보완', at: 3500 },
          { seq: 13, step: 'audit-2', text: '재점검', at: 4500 },
        ],
      }),
      status: 'running',
      tier: 'high',
      requests: [],
      now: 5000,
      finishedAt: null,
    });
    const rubric = view.stages.find((s) => s.key === 'rubric');
    expect(rubric?.rounds).toBe(2);
    expect(view.liveRound).toBe(2);
    expect(rubric?.groups.map((g) => g.kind)).toEqual(['narration', 'round', 'narration', 'round']);
    expect(rubric?.groups[1]).toMatchObject({ kind: 'round', round: 1, groups: [{ kind: 'narration', text: '점검' }] });
    expect(rubric?.groups[2]).toMatchObject({ kind: 'narration', text: '보완' });
    expect(view.stages.filter((s) => s.key !== 'rubric').map((s) => [s.key, s.rounds])).toEqual([
      ['classify', 0],
      ['description', 0],
      ['interpretation', 0],
      ['judgment', 0],
      ['stylistic', 0],
      ['delivery', 0],
    ]);
  });

  it('점검 스텝이 시작되면 항목이 없어도 소구역 상자가 서고 liveRound가 그 회차를 가리킨다', () => {
    const view = buildPassage({
      transcript: transcript({
        steps: [
          { name: 'rubric-0', seq: 1, startedAt: 0, completedAt: 1000 },
          { name: 'audit-1', seq: 2, startedAt: 1000, completedAt: null },
        ],
      }),
      status: 'running',
      tier: 'high',
      requests: [],
      now: 2000,
      finishedAt: null,
    });

    const rubric = view.stages.find((s) => s.key === 'rubric');
    expect(view.liveRound).toBe(1);
    expect(rubric?.groups).toEqual([{ kind: 'round', key: 'rubric:round:1:0', seq: 2, round: 1, elapsedMs: 1000, groups: [] }]);
  });

  it('revise 스텝이 흐르는 동안은 liveRound가 없다', () => {
    const view = buildPassage({
      transcript: transcript({
        steps: [
          { name: 'audit-1', seq: 1, startedAt: 0, completedAt: 1000 },
          { name: 'rubric-revise-1-0', seq: 3, startedAt: 1000, completedAt: null },
        ],
        turns: [{ seq: 2, step: 'audit-1', text: '점검', at: 500 }],
      }),
      status: 'running',
      tier: 'high',
      requests: [],
      now: 2000,
      finishedAt: null,
    });

    expect(view.liveRound).toBeNull();
    const rubric = view.stages.find((s) => s.key === 'rubric');
    expect(rubric?.groups.map((g) => g.kind)).toEqual(['round']);
  });

  it('같은 라운드가 끊겼다 이어져도 소구역 열쇠가 겹치지 않는다', () => {
    const view = buildPassage({
      transcript: transcript({
        steps: [
          { name: 'audit-1', seq: 1, startedAt: 0, completedAt: 1000 },
          { name: 'rubric-0', seq: 3, startedAt: 1000, completedAt: 2000 },
          { name: 'calibration-1-0', seq: 5, startedAt: 2000, completedAt: 3000 },
        ],
        turns: [
          { seq: 2, step: 'audit-1', text: '점검 시작', at: 500 },
          { seq: 4, step: 'rubric-0', text: '사이 발화', at: 1500 },
          { seq: 6, step: 'calibration-1-0', text: '점검 이어서', at: 2500 },
        ],
      }),
      status: 'running',
      tier: 'high',
      requests: [],
      now: 4000,
      finishedAt: null,
    });
    const rubric = view.stages.find((s) => s.key === 'rubric');
    expect(rubric?.groups.map((g) => g.kind)).toEqual(['round', 'narration', 'round']);
    expect(rubric?.rounds).toBe(1);
    const keys = rubric?.groups.map((g) => g.key) ?? [];
    expect(new Set(keys).size).toBe(keys.length);
    expect(keys.filter((key) => key.includes(':round:'))).toEqual(['rubric:round:1:0', 'rubric:round:1:1']);
  });

  it('단계 없는 스텝의 발화는 직전 단계에 얹히고, 스텝 없는 발화는 버려진다', () => {
    const view = buildPassage({
      transcript: transcript({
        steps: [
          { name: 'manuscript', seq: 2, startedAt: 0, completedAt: 1000 },
          { name: 'delivery-0', seq: 3, startedAt: 1000, completedAt: 2000 },
          { name: 'findings', seq: 5, startedAt: 2000, completedAt: null },
        ],
        turns: [
          { seq: 1, step: null, text: '아직 단계가 없어요', at: 100 },
          { seq: 4, step: 'delivery-0', text: '전할 말을 골랐어요', at: 1500 },
          { seq: 6, step: 'findings', text: '여백에 남길 말을 정리했어요', at: 2500 },
        ],
      }),
      status: 'running',
      tier: 'low',
      requests: [],
      now: 3000,
      finishedAt: null,
    });
    expect(view.stages.map((s) => [s.key, s.groups.length])).toEqual([
      ['classify', 0],
      ['judgment', 0],
      ['stylistic', 0],
      ['delivery', 2],
    ]);
    expect(view.stages.at(-1)?.groups).toMatchObject([
      { kind: 'narration', text: '전할 말을 골랐어요' },
      { kind: 'narration', text: '여백에 남길 말을 정리했어요' },
    ]);
    expect(view.stages.at(-1)?.summary).toBe('여백에 남길 말을 정리했어요');
  });

  it('첫 단계가 서기 전의 질문은 prelude에 선다', () => {
    const view = buildPassage({
      transcript: transcript({ steps: [{ name: 'manuscript', seq: 2, startedAt: 0, completedAt: null }] }),
      status: 'running',
      tier: 'medium',
      requests: [request(1, 'pending')],
      now: 3000,
      finishedAt: null,
    });
    expect(view.current).toBeNull();
    expect(view.prelude.map((g) => g.kind)).toEqual(['question']);
    expect(view.stages.every((stage) => stage.groups.length === 0)).toBe(true);
  });

  it('티어에 없는 단계에서 온 질문은 현재 단계 아래에 선다', () => {
    const view = buildPassage({
      transcript: transcript({
        steps: [
          { name: 'classify-0', seq: 1, startedAt: 0, completedAt: 1000 },
          { name: 'description-0', seq: 2, startedAt: 1000, completedAt: 2000 },
          { name: 'interpretation-0', seq: 3, startedAt: 2000, completedAt: null },
        ],
      }),
      status: 'running',
      tier: 'medium',
      requests: [request(4, 'pending')],
      now: 5000,
      finishedAt: null,
    });
    expect(view.current).toBe('description');
    expect(view.prelude).toEqual([]);
    expect(view.stages.find((stage) => stage.key === 'description')?.groups.map((g) => g.kind)).toEqual(['question']);
  });

  it('현재 단계는 마지막 스텝이 정한다 — 앞 단계로 되돌아가면 그 단계가 현재다', () => {
    const view = buildPassage({
      transcript: transcript({
        steps: [
          { name: 'classify-0', seq: 1, startedAt: 0, completedAt: 1000 },
          { name: 'description-0', seq: 2, startedAt: 1000, completedAt: 2000 },
          { name: 'classify-1', seq: 3, startedAt: 2000, completedAt: null },
        ],
      }),
      status: 'running',
      tier: 'medium',
      requests: [],
      now: 3000,
      finishedAt: null,
    });
    expect(view.current).toBe('classify');
    expect(view.stages.map((s) => s.status)).toEqual(['running', 'pending', 'pending', 'pending', 'pending']);
  });

  it('단계가 서기 전에 끊기면 첫 단계가 종결 상태를 든다', () => {
    const view = buildPassage({
      transcript: transcript({ steps: [{ name: 'manuscript', seq: 1, startedAt: 0, completedAt: null }] }),
      status: 'canceled',
      tier: 'low',
      requests: [],
      now: 5000,
      finishedAt: 3000,
    });
    expect(view.current).toBeNull();
    expect(view.stages.map((s) => s.status)).toEqual(['canceled', 'pending', 'pending', 'pending']);
    expect(view.stages[0].elapsedMs).toBeNull();
  });

  it('종결 뒤에는 now가 흘러도 경과가 얼어붙는다', () => {
    const settled = transcript({
      steps,
      turns: [{ seq: 4, step: 'description-0', text: '읽는 중', at: 30_000 }],
      tools: [tool(5, 'description-0', 'read', { path: 'manuscript/a.md', at: 40_000 })],
    });
    const at = (now: number, finishedAt: number | null, requests: ToolRequestMessage[] = []) =>
      buildPassage({ transcript: settled, status: 'canceled', tier: 'medium', requests, now, finishedAt });

    const soon = at(200_000, null);
    const later = at(86_400_000, null);
    expect(soon.elapsedMs).toBe(later.elapsedMs);
    expect(soon.stages[1].elapsedMs).toBe(later.stages[1].elapsedMs);
    expect(soon.elapsedMs).toBe(40_000);
    expect(soon.stages[1].elapsedMs).toBe(20_000);

    const fixed = at(86_400_000, 50_000);
    expect(fixed.elapsedMs).toBe(50_000);
    expect(fixed.stages[1].elapsedMs).toBe(30_000);

    const closed = at(86_400_000, 50_000, [request(6, 'closed', 35_000)]);
    expect(closed.elapsedMs).toBe(50_000 - (35_000 - 1006));
  });

  it('종결 상태는 현재 단계에 얹힌다', () => {
    const args = { transcript: transcript({ steps }), tier: 'medium' as const, requests: [], now: 200_000, finishedAt: null };
    const canceled = buildPassage({ ...args, status: 'canceled' });
    expect(canceled.stages.map((s) => s.status)).toEqual(['done', 'canceled', 'pending', 'pending', 'pending']);
    const failed = buildPassage({ ...args, status: 'failed' });
    expect(failed.stages[1].status).toBe('failed');
    const completed = buildPassage({ ...args, status: 'completed' });
    expect(completed.stages[1].status).toBe('done');
  });
});

describe('labels', () => {
  it('경과 문면', () => {
    expect(runningLabel(30_000)).toBe('방금');
    expect(runningLabel(250_000)).toBe('4분째');
    expect(spentLabel(30_000)).toBe('1분 미만');
    expect(spentLabel(540_000)).toBe('9분');
    expect(elapsedLabel(30_000)).toBe('방금 시작');
    expect(elapsedLabel(250_000)).toBe('4분');
  });
});
