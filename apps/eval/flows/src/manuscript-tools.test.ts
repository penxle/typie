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
