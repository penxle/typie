import { describe, expect, it } from 'vitest';
import { applyDelta, sealTurn, startTurn } from './delta.ts';
import type { TurnContext } from '@typie/prism';

const ctx: TurnContext = { agent: { id: 'a', name: 'assistant' }, run: 1, turn: 2, attempt: 1 };

describe('applyDelta', () => {
  it('offset 0은 스냅샷(덮어쓰기), 연속 offset은 이어 붙인다', () => {
    let live = applyDelta(null, { context: ctx, channel: 'text', offset: 0, data: '안녕' });
    live = applyDelta(live, { context: ctx, channel: 'text', offset: 2, data: '하세요' });
    expect(live.text).toBe('안녕하세요');
    live = applyDelta(live, { context: ctx, channel: 'text', offset: 0, data: '다시' });
    expect(live).toMatchObject({ text: '다시', textBroken: false });
  });
  it('offset이 건너뛰면 textBroken — 이후 조각은 무시, 스냅샷만 되살린다', () => {
    let live = applyDelta(null, { context: ctx, channel: 'text', offset: 0, data: '가' });
    live = applyDelta(live, { context: ctx, channel: 'text', offset: 5, data: '나' });
    expect(live).toMatchObject({ text: '가', textBroken: true });
    live = applyDelta(live, { context: ctx, channel: 'text', offset: 2, data: '다' });
    expect(live.text).toBe('가');
    live = applyDelta(live, { context: ctx, channel: 'text', offset: 0, data: '전체' });
    expect(live).toMatchObject({ text: '전체', textBroken: false });
  });
  it('thinking은 누적 글자 수, tool.input은 도구 이름을 받는다', () => {
    let live = applyDelta(null, { context: ctx, channel: 'thinking', chars: 12 });
    expect(live.thinkingChars).toBe(12);
    live = applyDelta(live, { context: ctx, channel: 'tool.input', tool: { id: null, name: 'read' } });
    expect(live.toolInput).toEqual({ name: 'read' });
  });
  it('다른 (agent,run,turn,attempt)의 조각은 상태를 통째로 갈아 끼운다', () => {
    const live = applyDelta(applyDelta(null, { context: ctx, channel: 'text', offset: 0, data: '가' }), {
      context: { ...ctx, turn: 3 },
      channel: 'text',
      offset: 0,
      data: '나',
    });
    expect(live).toMatchObject({ context: { turn: 3 }, text: '나' });
  });
});

describe('sealTurn / startTurn', () => {
  const live = applyDelta(null, { context: ctx, channel: 'text', offset: 0, data: '가' });
  it('같은 (agent, turn)의 turn.completed·turn 없는 종결은 봉인, 다른 턴은 유지', () => {
    expect(sealTurn(live, { agent: ctx.agent, run: 1, turn: 2, attempt: 1 })).toBeNull();
    expect(sealTurn(live, { agent: ctx.agent, run: 1 })).toBeNull();
    expect(sealTurn(live, { agent: ctx.agent, run: 1, turn: 1, attempt: 1 })).toBe(live);
  });
  it('startTurn은 지금 흐르는 턴의 시작만 통과시키고 나머지는 지운다', () => {
    expect(startTurn(live, { agent: ctx.agent, run: 1, turn: 2, attempt: 1 })).toBe(live);
    expect(startTurn(live, { agent: ctx.agent, run: 1, turn: 3, attempt: 1 })).toBeNull();
    expect(startTurn(null, { agent: ctx.agent, run: 1, turn: 3, attempt: 1 })).toBeNull();
  });
});
