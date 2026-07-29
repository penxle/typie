import { describe, expect, it } from 'vitest';
import { executeGrep, executeRead, READ_CAP } from './manuscript-tools.ts';

const CONTENT = '홍길동은 문을 열었다. 김영희가 고개를 들었다. 안내 방송. 다시 안내 방송이었다.';

describe('executeRead', () => {
  it('구간을 좌표와 함께 돌려준다', () => {
    const r = executeRead(CONTENT, 0, 10);
    expect(r).toEqual({ start: 0, end: 10, text: CONTENT.slice(0, 10), truncated: false });
  });

  it('범위를 본문 안으로 클램프한다', () => {
    const r = executeRead(CONTENT, -5, 10_000);
    expect(r.start).toBe(0);
    expect(r.end).toBe(CONTENT.length);
  });

  // 상한을 넘는 요청은 자르되, 잘렸음을 알려 에이전트가 이어서 읽게 한다.
  it('상한 초과는 자르고 truncated를 켠다', () => {
    const long = '가'.repeat(READ_CAP * 2);
    const r = executeRead(long, 0, READ_CAP * 2);
    expect(r.end).toBe(READ_CAP);
    expect(r.truncated).toBe(true);
  });
});

// 잘린 반쪽 서로게이트는 JSON 직렬화를 400으로 죽인다 — 어떤 결과에도 남으면 안 된다.
const wellFormed = (s: string) => {
  let i = 0;
  while (i < s.length) {
    const c = s.codePointAt(i) ?? 0;
    // BMP 밖 코드포인트는 온전한 쌍이다. 서로게이트 범위 값이 그대로 보이면 잘린 반쪽이다.
    if (c >= 0xd8_00 && c <= 0xdf_ff) return false;
    i += c > 0xff_ff ? 2 : 1;
  }
  return true;
};

describe('서로게이트 쌍 보호', () => {
  const EMOJI = `${'가'.repeat(10)}😭😭${'나'.repeat(10)}`;

  it('read 경계가 쌍 중간이면 물러난다 — 연속 창이 쌍을 잃지 않는다', () => {
    const first = executeRead(EMOJI, 0, 11);
    expect(wellFormed(first.text)).toBe(true);
    expect(first.end).toBe(10);
    const second = executeRead(EMOJI, 11, EMOJI.length);
    expect(wellFormed(second.text)).toBe(true);
    expect(first.text + second.text).toBe(EMOJI);
  });

  it('상한 절단도 쌍을 지킨다', () => {
    const long = '가'.repeat(READ_CAP - 1) + '😭' + '나'.repeat(10);
    const r = executeRead(long, 0, long.length);
    expect(r.truncated).toBe(true);
    expect(wellFormed(r.text)).toBe(true);
  });

  it('grep 문맥 창도 쌍을 지킨다', () => {
    // 두 본문 모두 문맥 경계(±40유닛)가 정확히 쌍 한가운데 떨어지도록 배치했다.
    const frontSplit = '가'.repeat(50) + '😭' + '나'.repeat(39) + '표적' + '다'.repeat(5);
    const backSplit = '표적' + '나'.repeat(39) + '😭' + '다'.repeat(5);
    for (const text of [frontSplit, backSplit]) {
      const r = executeGrep(text, '표적');
      expect(r.total).toBe(1);
      expect(wellFormed(r.matches[0].context)).toBe(true);
    }
  });
});

describe('executeGrep', () => {
  it('전 매치의 좌표와 주변 문맥을 준다', () => {
    const r = executeGrep(CONTENT, '안내 방송');
    expect(r.total).toBe(2);
    expect(r.matches[0].start).toBe(CONTENT.indexOf('안내 방송'));
    expect(r.matches[0].context).toContain('안내 방송');
    expect(r.error).toBeNull();
  });

  it('잘못된 정규식은 죽지 않고 오류를 돌려준다', () => {
    const r = executeGrep(CONTENT, '[미완성');
    expect(r.matches).toEqual([]);
    expect(r.error).toContain('정규식');
  });

  it('유니코드 플래그로 한국어를 다룬다', () => {
    expect(executeGrep(CONTENT, String.raw`문을\s*열`).total).toBe(1);
  });
});
