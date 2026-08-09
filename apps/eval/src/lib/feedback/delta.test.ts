import { describe, expect, it } from 'vitest';
import { applyDelta, sealTurn, startTurn } from './delta.ts';

const AG = { id: 'agent-1', name: 'reviewer' };
const OTHER = { id: 'agent-2', name: 'proofreader' };

// 델타 프레임에는 봉투가 없다 — data 라인이 곧 프레임이다(prism core/sse.ts의 deltaFrame).
const piece = (over: Record<string, unknown>) => ({ agent: AG, turn: 1, attempt: 1, channel: 'text', offset: 0, text: '', ...over });
const count = (over: Record<string, unknown>) => ({ agent: AG, turn: 1, attempt: 1, channel: 'thinking', chars: 0, ...over });

const feed = (frames: unknown[]) => frames.reduce<ReturnType<typeof applyDelta>>((state, frame) => applyDelta(state, frame), null);

describe('turn delta state', () => {
  it('조각을 offset 순서대로 이어 붙인다', () => {
    const s = feed([piece({ text: '첫 문장을 ' }), piece({ offset: 6, text: '이어 씁니다' })]);
    expect(s).toMatchObject({ agent: AG, turn: 1, attempt: 1, text: '첫 문장을 이어 씁니다', textBroken: false });
  });

  it('offset이 어긋나면 그 턴의 텍스트를 포기한다', () => {
    const broken = feed([piece({ text: '앞부분' }), piece({ offset: 99, text: '뒷부분' })]);
    expect(broken).toMatchObject({ text: '앞부분', textBroken: true });
    // 빠진 조각이 무엇이었는지 알 길이 없으므로 뒤이어 이어지는 조각으로도 복구하지 않는다.
    expect(applyDelta(broken, piece({ offset: 3, text: '뒷부분' }))).toMatchObject({ text: '앞부분', textBroken: true });
  });

  it('진행 중인 턴에 닿은 offset 0 조각은 이어 붙이지 않고 덮어쓴다', () => {
    // 허브는 (재)접속한 소비자에게 그 턴의 누적 전체를 offset 0으로 다시 낸다(prism/src/core/sse.ts의 재접속 스냅샷).
    const flowing = feed([piece({ text: '앞부분' }), piece({ offset: 3, text: '뒷부분' })]);
    expect(flowing).toMatchObject({ text: '앞부분뒷부분' });
    expect(applyDelta(flowing, piece({ text: '앞부분뒷부분더 왔다' }))).toMatchObject({ text: '앞부분뒷부분더 왔다', textBroken: false });

    // 파손 채널은 스냅샷에서 빠지므로, 스냅샷이 닿았다는 것은 이어붙일 원본이 성했다는 뜻이다.
    const broken = applyDelta(flowing, piece({ offset: 99, text: '어긋난 조각' }));
    expect(broken).toMatchObject({ textBroken: true });
    expect(applyDelta(broken, piece({ text: '앞부분뒷부분' }))).toMatchObject({ text: '앞부분뒷부분', textBroken: false });
  });

  it('카운트 프레임은 마지막 값으로 갱신한다', () => {
    const s = feed([count({ chars: 120 }), count({ chars: 340 }), count({ channel: 'tool.input', tool: 'write', chars: 12 })]);
    expect(s).toMatchObject({ thinkingChars: 340, toolInput: { tool: 'write', chars: 12 } });
  });

  it('루트의 생각은 조각으로 오지만 글자 수로 접는다', () => {
    const s = feed([piece({ channel: 'thinking', offset: 5, text: '조각난 생각' })]);
    expect(s).toMatchObject({ thinkingChars: 11, text: '', textBroken: false });
  });

  it('턴·시도·에이전트가 바뀌면 상태를 새로 연다', () => {
    const seeded = feed([piece({ text: '앞 턴의 말' }), count({ chars: 80 })]);
    expect(applyDelta(seeded, piece({ turn: 2, text: '새 턴' }))).toMatchObject({ turn: 2, text: '새 턴', thinkingChars: 0 });
    expect(applyDelta(seeded, piece({ attempt: 2, text: '재시도' }))).toMatchObject({ attempt: 2, text: '재시도', thinkingChars: 0 });
    expect(applyDelta(seeded, piece({ agent: OTHER, text: '다른 손' }))).toMatchObject({ agent: OTHER, text: '다른 손' });
  });

  it('봉인은 같은 턴에서만 상태를 지운다', () => {
    const seeded = feed([piece({ text: '흐르는 중' })]);
    expect(sealTurn(seeded, { agent: AG, turn: 1 })).toBeNull();
    expect(sealTurn(seeded, { agent: OTHER, turn: 1 })).toBe(seeded);
    expect(sealTurn(seeded, { agent: AG, turn: 2 })).toBe(seeded);
    // run 종결은 agent도 turn도 싣지 않는다 — 진행 중인 턴 없음의 권위 신호라 무조건 지운다.
    expect(sealTurn(seeded, {})).toBeNull();
    expect(sealTurn(null, { agent: AG, turn: 1 })).toBeNull();
  });

  it('턴의 시작은 다른 턴을 가리킬 때만 조각을 지운다', () => {
    const seeded = feed([piece({ text: '흐르는 중' })]);
    // 우리가 이미 그리고 있는 턴의 시작 — 조각은 그대로 산다(뒤이어 그 턴의 조각이 계속 온다).
    expect(startTurn(seeded, { agent: AG, turn: 1, attempt: 1 })).toBe(seeded);
    expect(startTurn(seeded, { agent: AG, turn: 2, attempt: 1 })).toBeNull();
    expect(startTurn(seeded, { agent: AG, turn: 1, attempt: 2 })).toBeNull();
    expect(startTurn(seeded, { agent: OTHER, turn: 1, attempt: 1 })).toBeNull();
    // 지목을 읽을 수 없는 프레임은 지운다 — 어느 턴의 시작인지 모른 채 옛 조각을 남기지 않는다.
    expect(startTurn(seeded, {})).toBeNull();
    expect(startTurn(null, { agent: AG, turn: 1, attempt: 1 })).toBeNull();
  });

  it('미지·깨진 프레임은 현 상태를 그대로 둔다', () => {
    const seeded = feed([piece({ text: '흐르는 중' })]);
    for (const frame of [
      null,
      'not a frame',
      piece({ channel: 'audio', text: '알 수 없는 채널' }),
      count({ channel: 'text' }),
      piece({ agent: { name: 'reviewer' } }),
      piece({ offset: '3' }),
      count({ chars: 'many' }),
    ]) {
      expect(applyDelta(seeded, frame)).toBe(seeded);
    }
    expect(applyDelta(null, 'not a frame')).toBeNull();
  });
});
