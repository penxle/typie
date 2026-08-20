import { describe, expect, it } from 'vitest';
import { Pacer } from './pacer.ts';

const run = (pacer: Pacer, ms: number) => {
  for (let elapsed = 16; elapsed <= ms; elapsed += 16) pacer.advance(16);
};

describe('pacer stream', () => {
  it('짧은 백로그는 읽기 속도(BASE_CPS) 근처의 연속 흐름으로 나온다', () => {
    const pacer = new Pacer();
    pacer.retarget('가나 '.repeat(100));
    run(pacer, 1000);
    expect(pacer.boundary).toBeGreaterThan(30);
    expect(pacer.boundary).toBeLessThan(90);
  });

  it('진행 중 뭉치는 점프하지 않고 가속으로 소화한다', () => {
    const pacer = new Pacer();
    pacer.retarget('가나다 '.repeat(75));
    expect(pacer.boundary).toBe(0);
    run(pacer, 200);
    expect(pacer.boundary).toBeGreaterThan(0);
    expect(pacer.boundary).toBeLessThan(300);
    run(pacer, 12_000);
    expect(pacer.boundary).toBe(300);
  });

  it('큰 백로그도 상한 안에서 가속할 뿐 한 번에 쏟지 않는다', () => {
    const pacer = new Pacer();
    pacer.retarget('가나 ');
    run(pacer, 500);
    pacer.retarget('가나 '.repeat(500));
    expect(pacer.boundary).toBeLessThan(10);
    run(pacer, 500);
    expect(pacer.boundary).toBeGreaterThan(10);
    expect(pacer.boundary).toBeLessThan(400);
  });

  it('합류 스냅샷(처음부터 크게 밀림)은 평문으로 점프한다', () => {
    const pacer = new Pacer();
    pacer.retarget('가'.repeat(500));
    expect(pacer.boundary).toBe(500);
    expect(pacer.plain).toBe(500);
  });

  it('축소는 내용 교체로 보고 그대로 점프한다', () => {
    const pacer = new Pacer();
    pacer.retarget('긴 문장이 흐르고 있었다');
    run(pacer, 2000);
    pacer.retarget('짧다');
    expect(pacer.boundary).toBe(2);
  });

  it('병리적 백로그(숨은 탭 복귀)는 점프한다', () => {
    const pacer = new Pacer();
    pacer.retarget('가나다 ');
    run(pacer, 500);
    pacer.retarget('가나다 ' + '나'.repeat(6000));
    expect(pacer.boundary).toBe(6004);
  });
});

describe('pacer finalize', () => {
  it('라이브에서 이어받아 전문을 끝까지 흘리고, 이미 보인 부분은 평문·꼬리만 페이드 대상이다', () => {
    const pacer = new Pacer();
    pacer.retarget('이미 보인 앞부분과');
    run(pacer, 1000);
    const shown = pacer.boundary;
    expect(shown).toBeGreaterThan(0);
    pacer.finalize('이미 보인 앞부분과 아직 안 보인 꼬리');
    expect(pacer.done).toBe(false);
    expect(pacer.plain).toBe(shown);
    run(pacer, 8000);
    expect(pacer.done).toBe(true);
    expect(pacer.plain).toBe(shown);
  });

  it('finalize 후에는 retarget이 무시된다', () => {
    const pacer = new Pacer();
    pacer.finalize('확정 전문');
    pacer.retarget('다른 내용으로 교체 시도');
    expect(pacer.text).toBe('확정 전문');
  });

  it('긴 꼬리도 쏟지 않고 흘린다', () => {
    const pacer = new Pacer();
    pacer.finalize('가'.repeat(300));
    run(pacer, 300);
    expect(pacer.boundary).toBeGreaterThan(0);
    expect(pacer.boundary).toBeLessThan(300);
    run(pacer, 12_000);
    expect(pacer.done).toBe(true);
  });

  it('이미 다 보인 전문은 곧바로 done이다', () => {
    const pacer = new Pacer();
    pacer.retarget('가'.repeat(500));
    pacer.finalize('가'.repeat(500));
    expect(pacer.done).toBe(true);
  });
});
