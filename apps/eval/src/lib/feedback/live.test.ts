import { describe, expect, it } from 'vitest';
import { applyEvent, capsuleLabel, groupFeed, initialLive, questionPausedMs } from './live.ts';
import type { FeedEntry, FeedGroup, QuestionEntry } from './live.ts';
import type { SseEvent } from './sse.ts';

// prism SSE의 data는 이벤트 본문이 아니라 {seq,kind,data,createdAt} 봉투다(prism core/sse.ts의 eventFrame).
const ev = (id: number, event: string, data: object, createdAt = 0): SseEvent => ({
  id,
  event,
  data: JSON.stringify({ seq: id, kind: event, data, createdAt }),
});

const AG = { id: 'agent-1', name: 'reviewer' };

// ask-user의 입력 전문은 JSON 문자열로 실린다(prism core/tool-dispatch.ts의 parkExternalToolCall) — 봉투 제약이
// 아니라 이미 나간 와이어 계약이 문자열이기 때문이다(live.ts의 parseAskUser가 그 계약의 소비자다). 파킹이
// 구체화한 질문 전문이 실리고, 해소(tool.called)의 사영에는 경로만 남는다(prism tools/standard.ts의 ask-user observe).
const ASK_INPUT = JSON.stringify({
  questions: [
    {
      question: '결말은 의도된 처리인가요?',
      hint: '답에 따라 비평 방향이 달라져요.',
      multi: false,
      options: [{ label: '의도한 열린 결말' }, { label: '퇴고 중 미완' }],
    },
  ],
});

const asked = (id: number, agentId = 'agent-a', at = 1000 + id) =>
  ev(
    id,
    'tool.requested',
    { agent: { id: agentId, name: 'plan' }, turn: 3, attempt: 1, tool: 'ask-user', toolCallId: 'call_1', input: ASK_INPUT },
    at,
  );

const answered = (id: number, agentId = 'agent-a', ok = true, at = 1000 + id) =>
  ev(
    id,
    'tool.called',
    {
      agent: { id: agentId, name: 'plan' },
      turn: 3,
      attempt: 1,
      tool: 'ask-user',
      input: { path: 'scratch/plan-high/questions-1.yaml' },
      ok,
    },
    at,
  );

describe('live reducer', () => {
  it('step 이벤트로 스테이지 상태를 전이한다', () => {
    let s = initialLive([]);
    s = applyEvent(s, ev(1, 'workflow.started', {}));
    s = applyEvent(s, ev(2, 'step.started', { step: 'research-0' }));
    expect(s.stages.research).toBe('running');
    s = applyEvent(s, ev(3, 'step.completed', { step: 'research-0' }));
    s = applyEvent(s, ev(4, 'step.started', { step: 'plan-0' }));
    expect(s.stages.research).toBe('done');
    expect(s.stages.plan).toBe('running');
    expect(s.stages.critique).toBe('pending');
  });

  it('표시 스테이지가 없는 step은 상태를 흔들지 않는다', () => {
    let s = initialLive([ev(1, 'step.started', { step: 'critique-0' })]);
    s = applyEvent(s, ev(2, 'step.started', { step: 'manuscript' }));
    expect(s.currentStage).toBe('critique');
    expect(s.currentStep).toBe('critique-0');
    expect(s.stages.critique).toBe('running');
  });

  it('turn.completed의 text를 발화 라인으로 접는다 — text null이면 라인 없음', () => {
    let s = initialLive([ev(1, 'step.started', { step: 'critique-0' })]);
    s = applyEvent(s, ev(2, 'turn.completed', { agent: AG, turn: 1, attempt: 1, text: '원고를 읽기 시작할게요', thinking: null }));
    s = applyEvent(s, ev(3, 'turn.completed', { agent: AG, turn: 2, attempt: 1, text: null, thinking: null }));
    expect(s.activity).toHaveLength(1);
    expect(s.activity[0]).toMatchObject({ id: 2, text: '원고를 읽기 시작할게요', stage: 'critique', step: 'critique-0' });
  });

  it('빈 문자열 text는 없는 것과 같다 — 라인을 만들지 않는다', () => {
    const s = initialLive([
      ev(1, 'step.started', { step: 'critique-0' }),
      ev(2, 'turn.completed', { agent: AG, turn: 1, attempt: 1, text: '', thinking: null }),
    ]);
    expect(s.activity).toHaveLength(0);
  });

  it('턴 발화도 진행 중 스테이지의 시각을 앞으로 민다', () => {
    const s = initialLive([
      ev(1, 'step.started', { step: 'critique-0' }, 1000),
      ev(2, 'turn.completed', { agent: AG, turn: 1, attempt: 1, text: '한 장을 다 읽었어요', thinking: null }, 6000),
    ]);
    expect(s.timing.critique).toEqual({ firstAt: 1000, lastAt: 6000 });
  });

  it('tool.called은 라인을 만들지 않고 진행 시각만 남긴다', () => {
    let s = initialLive([ev(1, 'step.started', { step: 'critique-0' }, 1000)]);
    s = applyEvent(s, ev(2, 'tool.called', { agent: AG, turn: 1, attempt: 1, tool: 'read', ok: true }, 5000));
    s = applyEvent(s, ev(3, 'tool.called', { agent: AG, turn: 1, attempt: 2, tool: 'read', ok: false }, 7000));
    expect(s.activity).toHaveLength(0);
    expect(s.timing.critique).toEqual({ firstAt: 1000, lastAt: 7000 });
  });

  it('tool.called은 캡슐 자취(mark)로 남는다 — 실패 호출과 list는 빠진다', () => {
    let s = initialLive([ev(1, 'step.started', { step: 'research-0' }, 1000)]);
    s = applyEvent(
      s,
      ev(2, 'tool.called', { agent: AG, turn: 1, attempt: 1, tool: 'read', input: { path: 'manuscript/01.txt' }, ok: true }, 2000),
    );
    s = applyEvent(
      s,
      ev(3, 'tool.called', { agent: AG, turn: 2, attempt: 1, tool: 'read', input: { path: 'artifacts/a.yaml' }, ok: true }, 3000),
    );
    s = applyEvent(
      s,
      ev(4, 'tool.called', { agent: AG, turn: 3, attempt: 1, tool: 'websearch', input: { query: '파쿠르 기술' }, ok: true }, 4000),
    );
    s = applyEvent(
      s,
      ev(
        5,
        'tool.called',
        { agent: AG, turn: 4, attempt: 1, tool: 'write', input: { path: 'artifacts/a.yaml', chars: 3819 }, ok: true },
        5000,
      ),
    );
    s = applyEvent(s, ev(6, 'tool.called', { agent: AG, turn: 5, attempt: 1, tool: 'list', input: {}, ok: true }, 6000));
    s = applyEvent(s, ev(7, 'tool.called', { agent: AG, turn: 6, attempt: 1, tool: 'grep', input: { pattern: 'x' }, ok: false }, 7000));
    expect(s.marks.map((mark) => mark.verb)).toEqual(['read-manuscript', 'read-note', 'search', 'write']);
    expect(s.marks[2]).toMatchObject({ query: '파쿠르 기술', chars: null });
    expect(s.marks[3]).toMatchObject({ query: null, chars: 3819 });
  });

  it('scratch/ 경로의 읽기·쓰기·고침은 임시 노트 동사로 갈린다', () => {
    let s = initialLive([ev(1, 'step.started', { step: 'research-0' }, 1000)]);
    const SCRATCH = 'scratch/research-high/questions-1.yaml';
    s = applyEvent(
      s,
      ev(2, 'tool.called', { agent: AG, turn: 1, attempt: 1, tool: 'write', input: { path: SCRATCH, chars: 812 }, ok: true }, 2000),
    );
    s = applyEvent(s, ev(3, 'tool.called', { agent: AG, turn: 2, attempt: 1, tool: 'read', input: { path: SCRATCH }, ok: true }, 3000));
    s = applyEvent(
      s,
      ev(
        4,
        'tool.called',
        { agent: AG, turn: 3, attempt: 1, tool: 'edit', input: { path: SCRATCH, oldBytes: 10, newBytes: 12 }, ok: true },
        4000,
      ),
    );
    expect(s.marks.map((mark) => mark.verb)).toEqual(['write-scratch', 'read-scratch', 'edit-scratch']);
    expect(capsuleLabel({ kind: 'tool', verb: 'write-scratch', query: null, chars: 812, count: 1 })).toBe('임시 노트 812자 작성함');
    expect(capsuleLabel({ kind: 'tool', verb: 'read-scratch', query: null, chars: null, count: 2 })).toBe('임시 노트 읽음 ×2');
    expect(capsuleLabel({ kind: 'tool', verb: 'edit-scratch', query: null, chars: null, count: 1 })).toBe('임시 노트 고침');
  });

  it('plan-review 스텝의 발화는 그 스텝에 귀속된다', () => {
    let s = initialLive([ev(1, 'step.started', { step: 'plan-0' }), ev(2, 'step.started', { step: 'plan-review-1' })]);
    s = applyEvent(
      s,
      ev(3, 'turn.completed', { agent: AG, turn: 4, attempt: 1, text: '고른 눈길이 서로 겹치지 않는지 봤어요', thinking: null }),
    );
    expect(s.activity.at(-1)).toMatchObject({ text: '고른 눈길이 서로 겹치지 않는지 봤어요', stage: 'plan', step: 'plan-review-1' });
  });

  it('활동 라인은 최근 200개만 유지한다', () => {
    let s = initialLive([ev(1, 'step.started', { step: 'critique-0' })]);
    for (let i = 0; i < 250; i++) {
      s = applyEvent(s, ev(i + 2, 'turn.completed', { agent: AG, turn: i + 1, attempt: 1, text: `기록 ${i}`, thinking: null }));
    }
    expect(s.activity).toHaveLength(200);
    expect(s.activity[0]?.text).toBe('기록 50');
    expect(s.activity.at(-1)?.text).toBe('기록 249');
  });

  it('활동 라인은 봉투의 createdAt을 시각으로 싣는다', () => {
    const s = initialLive([
      ev(1, 'step.started', { step: 'critique-0' }),
      ev(2, 'turn.completed', { agent: AG, turn: 1, attempt: 1, text: '앞 장의 복선을 되짚어 봤어요', thinking: null }, 1_700_000_123_000),
    ]);
    expect(s.activity.at(-1)?.at).toBe(1_700_000_123_000);
  });

  it('스테이지 전이는 활동 라인을 만들지 않는다', () => {
    const s = initialLive([ev(1, 'step.started', { step: 'critique-0' }), ev(2, 'step.started', { step: 'proofread-0' })]);
    expect(s.stages.critique).toBe('done');
    expect(s.activity).toHaveLength(0);
  });

  it('계획 검토·재수립 스텝은 하니스 라인을 만들지 않는다', () => {
    const s = initialLive([
      ev(1, 'step.started', { step: 'plan-0' }),
      ev(2, 'step.started', { step: 'plan-review-1-0' }),
      ev(3, 'step.started', { step: 'plan-revise-1' }),
    ]);
    expect(s.activity).toHaveLength(0);
    expect(s.stages.plan).toBe('running');
    expect(s.currentStep).toBe('plan-revise-1');
  });

  it('스테이지별 firstAt·lastAt을 봉투 시각으로 추적한다', () => {
    const s = initialLive([
      ev(1, 'workflow.started', {}, 1000),
      ev(2, 'step.started', { step: 'research-0' }, 2000),
      ev(3, 'tool.called', { agent: AG, turn: 1, attempt: 1, tool: 'read', ok: true }, 5000),
      ev(4, 'step.started', { step: 'plan-0' }, 9000),
    ]);
    expect(s.startedAt).toBe(1000);
    expect(s.timing.research).toEqual({ firstAt: 2000, lastAt: 9000 });
    expect(s.timing.plan).toEqual({ firstAt: 9000, lastAt: 9000 });
    expect(s.timing.critique).toEqual({ firstAt: null, lastAt: null });
  });

  it('터미널 이벤트를 관측하면 terminal이 선다', () => {
    const s = applyEvent(initialLive([]), ev(9, 'workflow.completed', { result: '{}' }));
    expect(s.terminal).toBe(true);
    expect(applyEvent(initialLive([]), ev(9, 'workflow.failed', { reason: 'boom' })).terminal).toBe(true);
    expect(applyEvent(initialLive([]), ev(9, 'workflow.canceled', {})).terminal).toBe(true);
  });

  it('workflow.completed는 진행 중이던 스테이지를 닫고, 실패·취소는 멈춘 자리를 남긴다', () => {
    const seeded = initialLive([ev(1, 'step.started', { step: 'conclude-0' }, 4000)]);

    const completed = applyEvent(seeded, ev(2, 'workflow.completed', { result: '{}' }, 7000));
    expect(completed.stages.conclude).toBe('done');
    expect(completed.timing.conclude).toEqual({ firstAt: 4000, lastAt: 7000 });

    expect(applyEvent(seeded, ev(2, 'workflow.failed', { reason: 'boom' })).stages.conclude).toBe('running');
    expect(applyEvent(seeded, ev(2, 'workflow.canceled', {})).stages.conclude).toBe('running');
  });

  it('커서는 마지막 id를 따른다', () => {
    const s = applyEvent(initialLive([]), ev(7, 'turn.started', {}));
    expect(s.cursor).toBe(7);
  });

  it('커서 이하의 재생분은 다시 접지 않는다', () => {
    const line = { agent: AG, turn: 1, attempt: 1, text: '원고를 처음부터 읽기 시작했어요', thinking: null };
    const seeded = initialLive([ev(1, 'step.started', { step: 'critique-0' }), ev(2, 'turn.completed', line)]);
    const replayed = applyEvent(seeded, ev(2, 'turn.completed', line));
    expect(replayed).toBe(seeded);
    expect(replayed.activity).toHaveLength(1);
  });

  it('깨진 프레임은 커서만 전진시킨다', () => {
    const s = applyEvent(initialLive([]), { id: 3, event: 'turn.completed', data: 'not json' });
    expect(s.activity).toHaveLength(0);
    expect(s.cursor).toBe(3);
  });

  it('검수 라운드의 자취는 발화와 무관하게 남는다 — 다음 스텝이 그 라운드를 닫는다', () => {
    const s = initialLive([
      ev(1, 'step.started', { step: 'plan-0' }, 1000),
      ev(2, 'step.started', { step: 'plan-review-1-0' }, 2000),
      ev(3, 'step.started', { step: 'plan-review-1-1' }, 60_000),
      ev(4, 'step.started', { step: 'plan-revise-1' }, 125_000),
    ]);
    expect(s.activity).toHaveLength(0);
    expect(s.nestedSpans[1]).toEqual({ id: 2, stage: 'plan', firstAt: 2000, lastAt: 125_000 });
  });

  it('종결 이벤트는 돌던 검수 라운드를 그 시각으로 닫는다', () => {
    const seeded = initialLive([ev(1, 'step.started', { step: 'plan-0' }, 1000), ev(2, 'step.started', { step: 'plan-review-2-0' }, 2000)]);
    expect(seeded.nestedSpans[2]).toEqual({ id: 2, stage: 'plan', firstAt: 2000, lastAt: 2000 });
    expect(applyEvent(seeded, ev(3, 'workflow.failed', { reason: 'boom' }, 8000)).nestedSpans[2]?.lastAt).toBe(8000);
    expect(applyEvent(seeded, ev(3, 'workflow.completed', { result: '{}' }, 9000)).nestedSpans[2]?.lastAt).toBe(9000);
  });

  it('시각 없는 봉투로 열린 라운드도 자취는 남는다 — 시간만 비어 있다', () => {
    const naked: SseEvent = {
      id: 1,
      event: 'step.started',
      data: JSON.stringify({ seq: 1, kind: 'step.started', data: { step: 'plan-review-1-0' } }),
    };
    expect(initialLive([naked]).nestedSpans[1]).toEqual({ id: 1, stage: 'plan', firstAt: null, lastAt: null });
  });
});

describe('질문 상태 기계', () => {
  it('tool.requested가 pending 엔트리를 세우고 스테이지·스텝에 귀속한다', () => {
    const s = initialLive([ev(1, 'step.started', { step: 'plan-0' }, 1000), asked(2)]);
    expect(s.questions).toHaveLength(1);
    const entry = s.questions[0];
    expect(entry).toMatchObject({
      agentId: 'agent-a',
      agentName: 'plan',
      toolCallId: 'call_1',
      stage: 'plan',
      step: 'plan-0',
      status: 'pending',
    });
    expect(entry.questions[0]).toMatchObject({
      question: '결말은 의도된 처리인가요?',
      hint: '답에 따라 비평 방향이 달라져요.',
      multi: false,
    });
    expect(entry.questions[0].options).toHaveLength(2);
  });

  it('같은 agent의 ask-user tool.called가 pending을 answered로 굳힌다 — ok:false여도', () => {
    for (const ok of [true, false]) {
      const s = initialLive([ev(1, 'step.started', { step: 'plan-0' }, 1000), asked(2), answered(3, 'agent-a', ok)]);
      expect(s.questions[0].status).toBe('answered');
    }
  });

  it('타 agent·타 도구의 tool.called는 pending을 건드리지 않는다', () => {
    const other = ev(
      3,
      'tool.called',
      { agent: { id: 'agent-b', name: 'plan' }, turn: 1, attempt: 1, tool: 'read', input: { path: 'manuscript/01.txt' }, ok: true },
      3000,
    );
    const s = initialLive([ev(1, 'step.started', { step: 'plan-0' }, 1000), asked(2), other, answered(4, 'agent-b')]);
    expect(s.questions[0].status).toBe('pending');
  });

  it('requested 없는 ok:false tool.called(파킹 전 검증 실패)는 무시된다', () => {
    const s = initialLive([ev(1, 'step.started', { step: 'plan-0' }, 1000), answered(2, 'agent-a', false)]);
    expect(s.questions).toHaveLength(0);
  });

  it('루트 종결이 pending을 closed로 굳힌다', () => {
    const seeded = initialLive([ev(1, 'step.started', { step: 'plan-0' }, 1000), asked(2)]);
    expect(applyEvent(seeded, ev(3, 'workflow.canceled', {}, 3000)).questions[0].status).toBe('closed');
    expect(applyEvent(seeded, ev(3, 'workflow.failed', { reason: 'boom' }, 3000)).questions[0].status).toBe('closed');
    expect(applyEvent(seeded, ev(3, 'workflow.completed', { result: '{}' }, 3000)).questions[0].status).toBe('closed');
  });

  it('순차 질문의 기록이 배열로 보존된다 — research 답변 후 plan 질문', () => {
    const s = initialLive([
      ev(1, 'step.started', { step: 'research-0' }, 1000),
      asked(2, 'agent-r'),
      answered(3, 'agent-r'),
      ev(4, 'step.started', { step: 'plan-0' }, 4000),
      asked(5, 'agent-p'),
    ]);
    expect(s.questions.map((q) => q.status)).toEqual(['answered', 'pending']);
    expect(s.questions.map((q) => q.stage)).toEqual(['research', 'plan']);
  });

  it('깨진 input은 엔트리를 만들지 않는다', () => {
    const broken = ev(
      2,
      'tool.requested',
      { agent: { id: 'agent-a', name: 'plan' }, turn: 1, attempt: 1, tool: 'ask-user', toolCallId: 'call_1', input: '{broken' },
      2000,
    );
    expect(initialLive([ev(1, 'step.started', { step: 'plan-0' }, 1000), broken]).questions).toHaveLength(0);
  });

  it('빈 문면(hint·question·label)도 엔트리를 세운다 — 카드가 없으면 파킹된 run은 답할 길이 없다', () => {
    const bare = ev(
      2,
      'tool.requested',
      {
        agent: { id: 'agent-a', name: 'plan' },
        turn: 1,
        attempt: 1,
        tool: 'ask-user',
        toolCallId: 'call_1',
        input: JSON.stringify({
          questions: [{ question: '', hint: '', multi: false, options: [{ label: '' }, { label: '퇴고 중 미완', description: '' }] }],
        }),
      },
      2000,
    );
    const s = initialLive([ev(1, 'step.started', { step: 'plan-0' }, 1000), bare]);
    expect(s.questions).toHaveLength(1);
    expect(s.questions[0].questions[0]).toMatchObject({ question: '', hint: '', multi: false });
    expect(s.questions[0].questions[0].options).toEqual([{ label: '' }, { label: '퇴고 중 미완' }]);
  });

  it('구조가 어긋난 질문은 여전히 엔트리를 만들지 않는다', () => {
    const malformed = (questions: unknown) =>
      ev(
        2,
        'tool.requested',
        {
          agent: { id: 'agent-a', name: 'plan' },
          turn: 1,
          attempt: 1,
          tool: 'ask-user',
          toolCallId: 'call_1',
          input: JSON.stringify({ questions }),
        },
        2000,
      );
    const seed = ev(1, 'step.started', { step: 'plan-0' }, 1000);
    expect(initialLive([seed, malformed([{ question: '물음', hint: '힌트', multi: 'no', options: [] }])]).questions).toHaveLength(0);
    expect(initialLive([seed, malformed([{ question: '물음', hint: '힌트', multi: false, options: '없음' }])]).questions).toHaveLength(0);
    expect(
      initialLive([seed, malformed([{ question: '물음', hint: '힌트', multi: false, options: [{ label: 3 }] }])]).questions,
    ).toHaveLength(0);
    expect(initialLive([seed, malformed([])]).questions).toHaveLength(0);
  });

  it('ask-user 아닌 도구의 tool.requested는 질문이 아니다', () => {
    const parked = ev(
      2,
      'tool.requested',
      {
        agent: { id: 'agent-a', name: 'plan' },
        turn: 1,
        attempt: 1,
        tool: 'read',
        toolCallId: 'call_2',
        input: '{"path":"manuscript/01.txt"}',
      },
      2000,
    );
    expect(initialLive([ev(1, 'step.started', { step: 'plan-0' }, 1000), parked]).questions).toHaveLength(0);
  });

  it('재생 복원 동형성 — 같은 이벤트열은 같은 상태를 만든다', () => {
    const events = [ev(1, 'step.started', { step: 'plan-0' }, 1000), asked(2), answered(3)];
    const replayed = initialLive(events);
    let incremental = initialLive([]);
    for (const event of events) incremental = applyEvent(incremental, event);
    expect(incremental.questions).toEqual(replayed.questions);
  });
});

// 카드 구조는 상태에서만 나온다 — 같은 이벤트를 다시 접으면 같은 모양이 다시 선다(재생·새로고침 동형).
describe('feed grouping', () => {
  const entryShape = (entry: FeedEntry) => {
    if (entry.kind === 'line') return entry.line.text;
    if (entry.kind === 'question') return `질문${entry.entry.questions.length}`;
    return `[${entry.items.map((item) => capsuleLabel(item)).join('·')}]`;
  };
  const shape = (groups: FeedGroup[]) =>
    groups.map((group) => (group.kind === 'nested' ? `검수${group.round}(${group.feed.map(entryShape).join('|')})` : entryShape(group)));

  const talk = (id: number, text: string, at: number) =>
    ev(id, 'turn.completed', { agent: AG, turn: id, attempt: 1, text, thinking: null }, at);

  const tool = (id: number, name: string, input: object, at: number) =>
    ev(id, 'tool.called', { agent: AG, turn: id, attempt: 1, tool: name, input, ok: true }, at);

  it('발화 없는 검수 라운드도 빈 카드로 선다 — 자리는 라운드가 시작한 순간이다', () => {
    const s = initialLive([
      ev(1, 'step.started', { step: 'plan-0' }, 1000),
      talk(2, '눈길 후보를 골라 적었어요', 2000),
      ev(3, 'step.started', { step: 'plan-review-1-0' }, 3000),
      ev(4, 'step.started', { step: 'plan-revise-1' }, 4000),
      talk(5, '겹치는 눈길 하나를 덜어냈어요', 5000),
    ]);
    expect(shape(groupFeed(s, 'plan'))).toEqual(['눈길 후보를 골라 적었어요', '검수1()', '겹치는 눈길 하나를 덜어냈어요']);

    const empty = groupFeed(s, 'plan')[1];
    expect(empty.kind === 'nested' && empty.span).toEqual({ id: 3, stage: 'plan', firstAt: 3000, lastAt: 4000 });
  });

  it('같은 라운드의 연속 발화는 한 카드로 접히고, 라운드가 갈리면 카드도 갈린다', () => {
    const s = initialLive([
      ev(1, 'step.started', { step: 'plan-review-1-0' }, 1000),
      talk(2, '첫 검수 발화', 2000),
      ev(3, 'step.started', { step: 'plan-review-1-1' }, 3000),
      talk(4, '이어진 검수 발화', 4000),
      ev(5, 'step.started', { step: 'plan-revise-1' }, 5000),
      talk(6, '되짚어 고쳐 적었어요', 6000),
      ev(7, 'step.started', { step: 'plan-review-2-0' }, 7000),
      talk(8, '두 번째 검수 발화', 8000),
    ]);
    expect(shape(groupFeed(s, 'plan'))).toEqual([
      '검수1(첫 검수 발화|이어진 검수 발화)',
      '되짚어 고쳐 적었어요',
      '검수2(두 번째 검수 발화)',
    ]);
  });

  it('질문은 제 커서 자리에 카드로 선다 — 라운드 안이면 그 카드 안이다', () => {
    const s = initialLive([
      ev(1, 'step.started', { step: 'plan-0' }, 1000),
      talk(2, '계획을 세우기 전에 하나 여쭐게요', 2000),
      asked(3, 'agent-a', 3000),
      talk(4, '답을 받아 계획을 적었어요', 4000),
      ev(5, 'step.started', { step: 'plan-review-1-0' }, 5000),
      asked(6, 'agent-a', 6000),
    ]);
    expect(shape(groupFeed(s, 'plan'))).toEqual(['계획을 세우기 전에 하나 여쭐게요', '질문1', '답을 받아 계획을 적었어요', '검수1(질문1)']);
  });

  it('질문을 기다린 시간은 생각으로 세지 않는다 — 사람이 기다린 틈이다', () => {
    const s = initialLive([
      ev(1, 'step.started', { step: 'plan-0' }, 0),
      talk(2, '하나 여쭐게요', 10_000),
      asked(3, 'agent-a', 12_000),
      talk(4, '답을 받아 계획을 적었어요', 732_000),
      tool(5, 'write', { path: 'artifacts/plan.yaml', chars: 1200 }, 733_000),
    ]);
    expect(shape(groupFeed(s, 'plan'))).toEqual(['하나 여쭐게요', '질문1', '답을 받아 계획을 적었어요', '[노트 1,200자 작성함]']);
  });

  it('라운드 자취는 제 스테이지 밖으로 새지 않는다', () => {
    const s = initialLive([
      ev(1, 'step.started', { step: 'plan-review-1-0' }, 1000),
      ev(2, 'step.started', { step: 'critique-0' }, 2000),
      talk(3, '원고를 처음부터 읽었어요', 3000),
    ]);
    expect(shape(groupFeed(s, 'plan'))).toEqual(['검수1()']);
    expect(shape(groupFeed(s, 'critique'))).toEqual(['원고를 처음부터 읽었어요']);
  });

  it('발화 사이의 연속 도구 자취가 캡슐로 접히고, 조용한 틈은 생각으로 선다', () => {
    const s = initialLive([
      ev(1, 'step.started', { step: 'research-0' }, 0),
      talk(2, '원고부터 읽어볼게요', 10_000),
      tool(3, 'read', { path: 'manuscript/01.txt' }, 12_000),
      tool(4, 'read', { path: 'manuscript/01.txt' }, 14_000),
      tool(5, 'read', { path: 'manuscript/01.txt' }, 59_000),
      talk(6, '이제 노트를 정리할게요', 61_000),
      tool(7, 'write', { path: 'artifacts/research.yaml', chars: 3819 }, 100_000),
    ]);
    expect(shape(groupFeed(s, 'research'))).toEqual([
      '원고부터 읽어볼게요',
      '[원고 읽음 ×3·45초 생각함]',
      '이제 노트를 정리할게요',
      '[노트 3,819자 작성함·39초 생각함]',
    ]);
  });

  it('발화 앞의 조용한 틈도 생각으로 선다 — 발화와 도구 호출은 같은 배치라 큰 틈은 발화 직전에 놓인다', () => {
    const s = initialLive([
      ev(1, 'step.started', { step: 'research-0' }, 0),
      talk(2, '원고를 읽었어요', 10_000),
      tool(3, 'read', { path: 'manuscript/01.txt' }, 11_000),
      talk(4, '검색으로 확인할게요', 56_000),
      tool(5, 'websearch', { query: '파쿠르' }, 57_000),
    ]);
    expect(shape(groupFeed(s, 'research'))).toEqual([
      '원고를 읽었어요',
      '[원고 읽음·45초 생각함]',
      '검색으로 확인할게요',
      '[‘파쿠르’ 검색함]',
    ]);
  });

  it('생각 시간은 60초를 넘으면 분·초로 갈아탄다', () => {
    const s = initialLive([
      ev(1, 'step.started', { step: 'research-0' }, 0),
      talk(2, '원고를 읽었어요', 10_000),
      talk(3, '구성을 정리했어요', 110_000),
      talk(4, '이제 노트를 쓸게요', 230_000),
    ]);
    expect(shape(groupFeed(s, 'research'))).toEqual([
      '원고를 읽었어요',
      '[1분 40초 생각함]',
      '구성을 정리했어요',
      '[2분 생각함]',
      '이제 노트를 쓸게요',
    ]);
  });

  it('검수 라운드 안의 도구 자취는 그 카드 안에 캡슐로 선다', () => {
    const s = initialLive([
      ev(1, 'step.started', { step: 'plan-review-1-0' }, 1000),
      tool(2, 'read', { path: 'artifacts/plan.yaml' }, 2000),
      talk(3, '계획을 검토했어요', 3000),
    ]);
    expect(shape(groupFeed(s, 'plan'))).toEqual(['검수1([노트 읽음]|계획을 검토했어요)']);
  });

  it('같은 상태를 다시 접어도 같은 구조가 선다', () => {
    const events = [
      ev(1, 'step.started', { step: 'plan-review-1-0' }, 1000),
      talk(2, '검수 발화', 2000),
      ev(3, 'step.started', { step: 'plan-review-2-0' }, 3000),
    ];
    expect(groupFeed(initialLive(events), 'plan')).toEqual(groupFeed(initialLive(events), 'plan'));
  });
});

describe('question paused time', () => {
  const frame = (id: number, kind: string, data: Record<string, unknown>, at: number): SseEvent => ({
    id,
    event: kind,
    data: JSON.stringify({ seq: id, kind, data, createdAt: at }),
  });
  const input = JSON.stringify({
    questions: [{ question: 'Q?', hint: 'h', multi: false, options: [{ label: '가' }, { label: '나' }] }],
  });
  const agent = { id: 'agent-a', name: 'plan' };
  const asked = (id: number, at: number) =>
    frame(id, 'tool.requested', { agent, turn: 1, attempt: 1, tool: 'ask-user', toolCallId: 'call_1', input }, at);

  const entry = (at: number | null, settledAt: number | null): QuestionEntry => ({
    id: 1,
    agentId: 'agent-a',
    agentName: 'plan',
    toolCallId: 'call_1',
    questions: [],
    stage: 'plan',
    step: 'plan-0',
    at,
    settledAt,
    status: settledAt === null ? 'pending' : 'answered',
  });

  it('해소·종결이 settledAt을 이벤트 시각으로 굳힌다', () => {
    const base = [frame(1, 'step.started', { step: 'plan-0' }, 1000), asked(2, 2000)];
    const resolved = initialLive([
      ...base,
      frame(3, 'tool.called', { agent, turn: 1, attempt: 1, tool: 'ask-user', input: { questions: ['Q?'] }, ok: true }, 62_000),
    ]);
    expect(resolved.questions[0]).toMatchObject({ status: 'answered', at: 2000, settledAt: 62_000 });

    const closed = initialLive([...base, frame(3, 'workflow.canceled', {}, 32_000)]);
    expect(closed.questions[0]).toMatchObject({ status: 'closed', settledAt: 32_000 });
  });

  it('파킹 구간 합산 — 해소분은 구간대로, 미해소는 now까지, at 없는 엔트리는 건너뛴다', () => {
    expect(questionPausedMs([entry(1000, 61_000)], 100_000)).toBe(60_000);
    expect(questionPausedMs([entry(1000, null)], 31_000)).toBe(30_000);
    expect(questionPausedMs([entry(null, null)], 31_000)).toBe(0);
    expect(questionPausedMs([entry(1000, 61_000), entry(70_000, null)], 80_000)).toBe(70_000);
  });

  it('창 겹침만 센다 — 창 밖 구간은 0, 걸친 구간은 잘라 센다', () => {
    const q = [entry(10_000, 70_000)];
    expect(questionPausedMs(q, 100_000, { from: 20_000, to: 50_000 })).toBe(30_000);
    expect(questionPausedMs(q, 100_000, { from: 80_000, to: 90_000 })).toBe(0);
    expect(questionPausedMs(q, 100_000, { from: 0, to: 10_000 })).toBe(0);
    expect(questionPausedMs(q, 100_000, { from: 40_000 })).toBe(30_000);
  });
});
