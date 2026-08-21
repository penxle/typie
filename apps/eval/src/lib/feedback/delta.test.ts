import { describe, expect, it } from 'vitest';
import { applyDelta, sealTurn, startTurn } from './delta.ts';

const AG = { id: 'agent-1', name: 'reviewer' };
const OTHER = { id: 'agent-2', name: 'proofreader' };

// 델타 프레임에는 로그 봉투가 없다 — data 라인이 곧 {context, channel, offset, data(, tool)}다(prism docs/events.md §8).
const ctx = (agent = AG, turn = 1, attempt = 1) => ({ agent, run: 1, turn, attempt });
const piece = (over: Record<string, unknown>) => ({ context: ctx(), channel: 'text', offset: 0, data: '', ...over });

const feed = (frames: unknown[]) => frames.reduce<ReturnType<typeof applyDelta>>((state, frame) => applyDelta(state, frame), null);

describe('turn delta state', () => {
  it('조각을 offset 순서대로 이어 붙인다', () => {
    const s = feed([piece({ data: '첫 문장을 ' }), piece({ offset: 6, data: '이어 씁니다' })]);
    expect(s).toMatchObject({ agent: AG, turn: 1, attempt: 1, text: '첫 문장을 이어 씁니다', textBroken: false });
  });

  it('offset이 어긋나면 그 턴의 텍스트를 포기한다', () => {
    const broken = feed([piece({ data: '앞부분' }), piece({ offset: 99, data: '뒷부분' })]);
    expect(broken).toMatchObject({ text: '앞부분', textBroken: true });
    // 빠진 조각이 무엇이었는지 알 길이 없으므로 뒤이어 이어지는 조각으로도 복구하지 않는다.
    expect(applyDelta(broken, piece({ offset: 3, data: '뒷부분' }))).toMatchObject({ text: '앞부분', textBroken: true });
  });

  it('진행 중인 턴에 닿은 offset 0 조각은 이어 붙이지 않고 덮어쓴다', () => {
    // 허브는 (재)접속한 소비자에게 그 턴의 누적 전체를 offset 0으로 다시 낸다(§8).
    const flowing = feed([piece({ data: '앞부분' }), piece({ offset: 3, data: '뒷부분' })]);
    expect(flowing).toMatchObject({ text: '앞부분뒷부분' });
    expect(applyDelta(flowing, piece({ data: '앞부분뒷부분더 왔다' }))).toMatchObject({ text: '앞부분뒷부분더 왔다', textBroken: false });

    // 파손 채널은 스냅샷에서 빠지므로, 스냅샷이 닿았다는 것은 이어붙일 원본이 성했다는 뜻이다.
    const broken = applyDelta(flowing, piece({ offset: 99, data: '어긋난 조각' }));
    expect(broken).toMatchObject({ textBroken: true });
    expect(applyDelta(broken, piece({ data: '앞부분뒷부분' }))).toMatchObject({ text: '앞부분뒷부분', textBroken: false });
  });

  it('생각·도구 인자 조각은 누적 길이(offset + 조각)로 접는다', () => {
    const s = feed([
      piece({ channel: 'thinking', offset: 0, data: 'x'.repeat(120) }),
      piece({ channel: 'thinking', offset: 120, data: 'y'.repeat(220) }),
      piece({ channel: 'tool.input', offset: 5, data: '"path"', tool: { id: 'call_1', name: 'write' } }),
    ]);
    expect(s).toMatchObject({ thinkingChars: 340, toolInput: { tool: 'write', chars: 11 }, text: '', textBroken: false });
  });

  it('도구 인자 조각은 파싱하지 않고 호출 이름만 읽는다 — id 없는 벤더도 이름은 있다', () => {
    const s = feed([piece({ channel: 'tool.input', offset: 0, data: '{"pa', tool: { id: null, name: 'ask-user' } })]);
    expect(s).toMatchObject({ toolInput: { tool: 'ask-user', chars: 4 } });
  });

  it('턴·시도·에이전트가 바뀌면 상태를 새로 연다', () => {
    const seeded = feed([piece({ data: '앞 턴의 말' }), piece({ channel: 'thinking', offset: 0, data: 'x'.repeat(80) })]);
    expect(applyDelta(seeded, piece({ context: ctx(AG, 2), data: '새 턴' }))).toMatchObject({ turn: 2, text: '새 턴', thinkingChars: 0 });
    expect(applyDelta(seeded, piece({ context: ctx(AG, 1, 2), data: '재시도' }))).toMatchObject({
      attempt: 2,
      text: '재시도',
      thinkingChars: 0,
    });
    expect(applyDelta(seeded, piece({ context: ctx(OTHER), data: '다른 손' }))).toMatchObject({ agent: OTHER, text: '다른 손' });
  });

  it('봉인은 같은 턴에서만 상태를 지운다 — 좌표는 봉투의 context다', () => {
    const seeded = feed([piece({ data: '흐르는 중' })]);
    expect(sealTurn(seeded, ctx(AG, 1))).toBeNull();
    expect(sealTurn(seeded, ctx(OTHER, 1))).toBe(seeded);
    expect(sealTurn(seeded, ctx(AG, 2))).toBe(seeded);
    // 워크플로 종결의 context는 비어 있다(§4) — 진행 중인 턴 없음의 권위 신호라 무조건 지운다.
    expect(sealTurn(seeded, {})).toBeNull();
    expect(sealTurn(null, ctx(AG, 1))).toBeNull();
  });

  it('턴의 시작은 다른 턴을 가리킬 때만 조각을 지운다', () => {
    const seeded = feed([piece({ data: '흐르는 중' })]);
    // 우리가 이미 그리고 있는 턴의 시작 — 조각은 그대로 산다(뒤이어 그 턴의 조각이 계속 온다).
    expect(startTurn(seeded, ctx(AG, 1, 1))).toBe(seeded);
    expect(startTurn(seeded, ctx(AG, 2, 1))).toBeNull();
    expect(startTurn(seeded, ctx(AG, 1, 2))).toBeNull();
    expect(startTurn(seeded, ctx(OTHER, 1, 1))).toBeNull();
    // 지목을 읽을 수 없는 프레임은 지운다 — 어느 턴의 시작인지 모른 채 옛 조각을 남기지 않는다.
    expect(startTurn(seeded, {})).toBeNull();
    expect(startTurn(seeded, undefined)).toBeNull();
    expect(startTurn(null, ctx(AG, 1, 1))).toBeNull();
  });

  it('미지·깨진 프레임은 현 상태를 그대로 둔다', () => {
    const seeded = feed([piece({ data: '흐르는 중' })]);
    for (const frame of [
      null,
      'not a frame',
      piece({ channel: 'audio', data: '알 수 없는 채널' }),
      piece({ context: { agent: { name: 'reviewer' }, run: 1, turn: 1, attempt: 1 } }),
      piece({ context: { agent: AG, run: 1, turn: 1 } }),
      piece({ offset: '3' }),
      piece({ data: 12 }),
      { context: ctx(), channel: 'text', offset: 0 },
    ]) {
      expect(applyDelta(seeded, frame)).toBe(seeded);
    }
    expect(applyDelta(null, 'not a frame')).toBeNull();
  });
});
