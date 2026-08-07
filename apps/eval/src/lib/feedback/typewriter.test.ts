import { describe, expect, it } from 'vitest';
import { Typewriter } from './typewriter.ts';

// 시간을 주입해 프레임을 결정적으로 돌린다 — 16ms 프레임, from~to(ms).
const run = (tw: Typewriter, from: number, to: number) => {
  for (let now = from + 16; now <= to; now += 16) tw.advance(now, 16);
};

const midWord = (text: string, cursor: number) =>
  cursor > 0 && cursor < text.length && /\S/.test(text[cursor]) && /\S/.test(text[cursor - 1]);

describe('typewriter stream', () => {
  it('진행 중 뭉치는 점프하지 않고 가속으로 소화한다', () => {
    const tw = new Typewriter('stream');
    tw.retarget('가나다 '.repeat(75), 0); // 300자 — HARD_LAG 아래, revealed 0이지만 JOIN_LAG 아래
    expect(tw.boundary).toBe(0); // 즉시 점프하지 않는다
    run(tw, 0, 200);
    expect(tw.boundary).toBeGreaterThan(0);
    expect(tw.boundary).toBeLessThan(300); // 한 번에 쏟아지지 않는다
    run(tw, 200, 4000);
    expect(tw.boundary).toBe(300); // 유휴 전환 후 꼬리까지 소진된다
  });

  it('유휴 전에는 공개 경계가 단어를 반으로 가르지 않는다', () => {
    // 유휴(450ms) 이후는 보류 해제로 글자 단위 흐름이 설계다 — 불변식은 상류가 살아 있는 동안의 것이다.
    const text = `${'하늘 바다 구름 '.repeat(20)}마지막문장`;
    const tw = new Typewriter('stream');
    tw.retarget(text, 0);
    for (let now = 16; now <= 440; now += 16) {
      tw.advance(now, 16);
      expect(midWord(text, tw.boundary)).toBe(false);
    }
    expect(tw.boundary).toBeGreaterThan(0);
  });

  it('버퍼 끝 미완 단어는 잡아두다가 유휴가 지나면 푼다', () => {
    const tw = new Typewriter('stream');
    tw.retarget('안녕하세요 세계입', 0); // 끝 단어가 미완일 수 있는 형태
    run(tw, 0, 400); // 아직 유휴 전(450ms) — 성장 시각 0 기준
    expect(tw.boundary).toBeLessThanOrEqual(6); // "안녕하세요 "까지만 — 끝 단어는 보류
    run(tw, 400, 2000); // 유휴 경과 — 보류 해제
    expect(tw.boundary).toBe(9);
  });

  it('합류 스냅샷(처음부터 크게 밀림)은 평문으로 점프한다', () => {
    const tw = new Typewriter('stream');
    tw.retarget('가'.repeat(500), 0);
    expect(tw.boundary).toBe(500);
    expect(tw.tokens().every((token) => !token.animated)).toBe(true);
  });

  it('축소는 내용 교체로 보고 그대로 점프한다', () => {
    const tw = new Typewriter('stream');
    tw.retarget('긴 문장이 흐르고 있었다', 0);
    run(tw, 0, 2000);
    tw.retarget('짧다', 2100);
    expect(tw.boundary).toBe(2);
  });

  it('병리적 백로그(숨은 탭 복귀)는 점프한다', () => {
    const tw = new Typewriter('stream');
    tw.retarget('가나다 ', 0);
    run(tw, 0, 500);
    tw.retarget('가나다 ' + '나'.repeat(3000), 600);
    expect(tw.boundary).toBe(3004);
  });
});

describe('typewriter final', () => {
  it('물려받은 위치부터 전문을 끝까지 흘리고, 승계 앞부분은 평문이다', () => {
    const text = '이미 보인 앞부분과 아직 안 보인 꼬리';
    const tw = new Typewriter('final', { text, from: 10, rate: 100, now: 0 });
    expect(tw.done).toBe(false);
    expect(tw.boundary).toBe(10);
    run(tw, 0, 3000);
    expect(tw.done).toBe(true);
    expect(tw.boundary).toBe(text.length);
    for (const token of tw.tokens()) {
      expect(token.animated).toBe(token.text.trim().length > 0 && token.start >= 10);
    }
  });

  it('긴 꼬리도 쏟지 않고 평시 지평으로 흘린다', () => {
    const text = '가'.repeat(300);
    const tw = new Typewriter('final', { text, from: 0, rate: 15, now: 0 });
    run(tw, 0, 300);
    expect(tw.boundary).toBeGreaterThan(0);
    expect(tw.boundary).toBeLessThan(300); // 한 번에 쏟아지지 않는다
    run(tw, 300, 6000);
    expect(tw.done).toBe(true);
  });

  it('이미 다 보인 전문은 곧바로 done이다', () => {
    const tw = new Typewriter('final', { text: '전부 보임', from: 5, rate: 15, now: 0 });
    expect(tw.done).toBe(true);
  });
});
