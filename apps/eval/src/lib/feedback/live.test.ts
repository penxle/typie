import { describe, expect, it } from 'vitest';
import { applyEvent, capsuleLabel, groupFeed, initialLive } from './live.ts';
import type { FeedEntry, FeedGroup } from './live.ts';
import type { SseEvent } from './sse.ts';

// prism SSE의 data는 이벤트 본문이 아니라 {seq,kind,data,createdAt} 봉투다(prism/src/session/sse.ts:49).
const ev = (id: number, event: string, data: object, createdAt = 0): SseEvent => ({
  id,
  event,
  data: JSON.stringify({ seq: id, kind: event, data, createdAt }),
});

const AG = { id: 'agent-1', name: 'reviewer' };

describe('live reducer', () => {
  it('step 이벤트로 스테이지 상태를 전이한다', () => {
    let s = initialLive([]);
    s = applyEvent(s, ev(1, 'run.started', { run: 1 }));
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
      ev(1, 'run.started', { run: 1 }, 1000),
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
    const s = applyEvent(initialLive([]), ev(9, 'run.completed', { run: 1 }));
    expect(s.terminal).toBe(true);
    expect(applyEvent(initialLive([]), ev(9, 'run.failed', { run: 1, reason: 'boom' })).terminal).toBe(true);
    expect(applyEvent(initialLive([]), ev(9, 'run.canceled', { run: 1 })).terminal).toBe(true);
  });

  it('run.completed는 진행 중이던 스테이지를 닫고, 실패·취소는 멈춘 자리를 남긴다', () => {
    const seeded = initialLive([ev(1, 'step.started', { step: 'conclude-0' }, 4000)]);

    const completed = applyEvent(seeded, ev(2, 'run.completed', { run: 1 }, 7000));
    expect(completed.stages.conclude).toBe('done');
    expect(completed.timing.conclude).toEqual({ firstAt: 4000, lastAt: 7000 });

    expect(applyEvent(seeded, ev(2, 'run.failed', { run: 1, reason: 'boom' })).stages.conclude).toBe('running');
    expect(applyEvent(seeded, ev(2, 'run.canceled', { run: 1 })).stages.conclude).toBe('running');
  });

  it('자식 run의 종결은 루트를 닫지 않는다 — agent가 실린 종결은 커서만 민다', () => {
    // 중계된 자식 이벤트에는 agent가 찍히고(prism propagation.ts:287), 루트 자신의 종결에는 없다(terminal.ts:82-84).
    const seeded = initialLive([ev(1, 'step.started', { step: 'plan-review-2-0' }, 1000)]);

    const child = applyEvent(seeded, ev(2, 'run.completed', { run: 1, agent: AG }, 8000));
    expect(child.terminal).toBe(false);
    expect(child.stages.plan).toBe('running');
    expect(child.nestedSpans[2]?.lastAt).toBe(1000);
    expect(child.timing.plan).toEqual({ firstAt: 1000, lastAt: 1000 });
    expect(child.cursor).toBe(2);

    expect(applyEvent(seeded, ev(2, 'run.failed', { run: 1, reason: 'boom', agent: AG }, 8000)).terminal).toBe(false);
    expect(applyEvent(seeded, ev(2, 'run.canceled', { run: 1, agent: AG }, 8000)).terminal).toBe(false);

    // 루트의 종결은 자식 종결을 지나온 뒤에도 그대로 닫는다.
    const root = applyEvent(child, ev(3, 'run.completed', { run: 1 }, 9000));
    expect(root.terminal).toBe(true);
    expect(root.stages.plan).toBe('done');
    expect(root.nestedSpans[2]?.lastAt).toBe(9000);
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
    expect(applyEvent(seeded, ev(3, 'run.failed', { run: 1, reason: 'boom' }, 8000)).nestedSpans[2]?.lastAt).toBe(8000);
    expect(applyEvent(seeded, ev(3, 'run.completed', { run: 1 }, 9000)).nestedSpans[2]?.lastAt).toBe(9000);
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

// 카드 구조는 상태에서만 나온다 — 같은 이벤트를 다시 접으면 같은 모양이 다시 선다(재생·새로고침 동형).
describe('feed grouping', () => {
  const entryShape = (entry: FeedEntry) =>
    entry.kind === 'line' ? entry.line.text : `[${entry.items.map((item) => capsuleLabel(item)).join('·')}]`;
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
      '[원고 읽음 ×2·45초 생각함·원고 읽음]',
      '이제 노트를 정리할게요',
      '[39초 생각함·노트 3,819자 작성함]',
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
