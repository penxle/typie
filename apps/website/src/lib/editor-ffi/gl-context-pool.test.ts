import { describe, expect, it } from 'vitest';
import { GL_POOL_BUDGET, GlContextPool } from './gl-context-pool';
import type { LeaseToken, PageZone, SurfaceBackend } from './gl-context-pool';

type Change = [object, number, SurfaceBackend, LeaseToken | undefined];
type Timer = { at: number; fn: () => void };

const setup = (budget = 2, now: () => number = () => 1000) => {
  const changes: Change[] = [];
  const timers: Timer[] = [];
  const schedule = (fn: () => void, ms: number) => {
    const timer = { at: now() + ms, fn };
    timers.push(timer);
    return () => {
      const index = timers.indexOf(timer);
      if (index !== -1) timers.splice(index, 1);
    };
  };
  const pool = new GlContextPool(
    budget,
    (e, p, b, hint) => {
      changes.push([e, p, b, hint]);
    },
    now,
    schedule,
  );
  return { pool, changes, timers };
};

const fireTimers = (timers: Timer[]): void => {
  const due = [...timers];
  timers.length = 0;
  for (const timer of due) timer.fn();
};

// 정상 경로를 끝까지 밀어붙여(acquire+ack) 토큰을 돌려준다. cpu로 대체되면 테스트 전제가 깨진
// 것이므로 원인을 숨기지 않고 바로 실패시킨다.
const expectGlLease = (pool: GlContextPool, ed: object, page: number, zone: PageZone): LeaseToken => {
  const demand = pool.updatePageDemand(ed, page, zone);
  if (demand !== 'gl') throw new Error('테스트 전제 위반: gl 배정이 아니다');
  const lease = pool.acquireCanvasLease(ed, page, 'gl');
  if (lease.backend !== 'gl') throw new Error('테스트 전제 위반: 예약이 거부됐다');
  pool.ackAttached(lease.token, 'gl');
  return lease.token;
};

// 콜백으로 이미 acquireHint가 와 있으면 그것을 소비하고(자체 재획득 금지), 없으면
// acquireCanvasLease를 직접 호출한다 — 실패 후 재시도 사이클을 몇 번이고 반복하기 위한 헬퍼.
const claimLease = (pool: GlContextPool, changes: Change[], ed: object, page: number, zone: PageZone): LeaseToken => {
  const hintIndex = changes.findIndex(([e, p, b]) => e === ed && p === page && b === 'gl');
  if (hintIndex !== -1) {
    const hint = changes[hintIndex][3];
    changes.splice(hintIndex, 1);
    if (hint === undefined) throw new Error('gl 승격 콜백에는 acquireHint가 있어야 한다');
    return hint;
  }
  const demand = pool.updatePageDemand(ed, page, zone);
  if (demand !== 'gl') throw new Error('테스트 전제 위반: gl 배정이 아니다');
  const lease = pool.acquireCanvasLease(ed, page, 'gl');
  if (lease.backend !== 'gl') throw new Error('테스트 전제 위반: 예약이 거부됐다');
  return lease.token;
};

const failAndRewin = (
  pool: GlContextPool,
  changes: Change[],
  timers: Timer[],
  advance: (ms: number) => void,
  ed: object,
  page: number,
  zone: PageZone,
): void => {
  pool.ackAttached(claimLease(pool, changes, ed, page, zone), 'cpu');
  advance(1500);
  fireTimers(timers);
};

describe('GlContextPool', () => {
  it('프로덕션 풀 예산은 8이다 (스펙 §3.4, 구 공유 presenter 제거 후)', () => {
    expect(GL_POOL_BUDGET).toBe(8);
  });

  it('초기 배정은 콜백 없이 반환되며, 실제 보유는 acquireCanvasLease+ack로만 생긴다', () => {
    const { pool, changes } = setup(2);
    const ed = {};
    expect(pool.updatePageDemand(ed, 0, 'visible')).toBe('gl');
    expect(pool.updatePageDemand(ed, 1, 'visible')).toBe('gl');
    expect(pool.updatePageDemand(ed, 2, 'visible')).toBe('cpu');
    expect(changes).toEqual([]);
    expect(pool.debugHoldCount()).toBe(0);

    expectGlLease(pool, ed, 0, 'visible');
    expectGlLease(pool, ed, 1, 'visible');
    expect(pool.debugHoldCount()).toBe(2);
  });

  it('예산 소진 시 승격은 released ack 이후에만 일어난다', () => {
    const { pool, changes } = setup(1);
    const ed = {};
    const firstToken = expectGlLease(pool, ed, 0, 'visible');
    expect(pool.updatePageDemand(ed, 1, 'visible')).toBe('cpu');

    changes.length = 0;
    pool.setFocus(ed);
    pool.leave(ed, 0);
    expect(pool.backendOf(ed, 1)).toBe('cpu');
    expect(pool.debugHoldCount()).toBe(1);

    pool.ackReleased(firstToken);
    expect(pool.backendOf(ed, 1)).toBe('gl');
    expect(changes.some(([e, p, b]) => e === ed && p === 1 && b === 'gl')).toBe(true);
    expect(pool.debugHoldCount()).toBe(1);
  });

  it('포커스 우선순위로 tier를 이겨도, 강등된 페이지의 lease가 released ack되기 전까지는 gl을 얻지 못한다', () => {
    const { pool, changes } = setup(2);
    const a = {};
    const b = {};
    pool.setFocus(b);
    expectGlLease(pool, a, 0, 'visible');
    const secondAToken = expectGlLease(pool, a, 1, 'visible');

    // focused(tier 1)가 unfocused visible(tier 2) a:1을 눌러 강등시키지만, 슬롯이 아직 안 빠졌으므로
    // 정책 배정은 cpu로 유보한다(원장 오염 방지) — 'gl'을 미리 찍어두면 슬롯이 빌 때 승격 루프가
    // backend==='gl'을 보고 건너뛰어 승격 콜백을 영영 안 내보낸다.
    expect(pool.updatePageDemand(b, 0, 'overscan')).toBe('cpu');
    expect(pool.backendOf(b, 0)).toBe('cpu');
    expect(pool.backendOf(a, 1)).toBe('cpu'); // 강등 자체는 이미 반영됨

    changes.length = 0;
    pool.beginRelease(secondAToken);
    pool.ackReleased(secondAToken); // 슬롯 해방 → rebalance가 b로 승격 콜백을 낸다

    // released ack 시점에만 b가 실제 gl 슬롯을 얻는다 — 재시도가 아니라 승격 콜백(acquireHint 포함)으로.
    const promotion = changes.find(([e, p, backend]) => e === b && p === 0 && backend === 'gl');
    expect(promotion).toBeDefined();
    expect(promotion?.[3]).toBeDefined(); // acquireHint(예약 토큰)가 콜백에 실려 온다
    expect(pool.backendOf(b, 0)).toBe('gl');
  });

  it('이미 해제되거나 취소된 토큰의 ack는 재획득된 lease에 영향을 주지 않는다', () => {
    const { pool } = setup(1);
    const ed = {};
    const first = expectGlLease(pool, ed, 0, 'visible');
    pool.beginRelease(first);
    pool.ackReleased(first);

    const second = expectGlLease(pool, ed, 0, 'visible');
    expect(second).not.toBe(first);

    pool.ackReleased(first); // stale/중복 — 무시
    expect(pool.backendOf(ed, 0)).toBe('gl');
    expect(pool.debugHoldCount()).toBe(1);

    pool.ackAttached(first, 'cpu'); // 이미 사라진 토큰 — 무시
    expect(pool.backendOf(ed, 0)).toBe('gl');
    expect(pool.debugHoldCount()).toBe(1);
  });

  it('ack 전 취소는 예약을 원자적으로 소거하고 홀드를 남기지 않는다', () => {
    const { pool } = setup(1);
    const ed = {};
    pool.updatePageDemand(ed, 0, 'visible');
    const lease = pool.acquireCanvasLease(ed, 0, 'gl');
    if (lease.backend !== 'gl') throw new Error('테스트 전제 위반');
    expect(pool.debugHoldCount()).toBe(1);

    pool.cancelReservation(lease.token, 'left-viewport');
    expect(pool.debugHoldCount()).toBe(0);
    expect(pool.backendOf(ed, 0)).toBe('gl'); // 정책 배정 자체는 유지 — 필요하면 재획득

    pool.ackAttached(lease.token, 'gl'); // 취소된 토큰의 뒤늦은 ack는 무시
    expect(pool.debugHoldCount()).toBe(0);
  });

  it('GL 예약 뒤 ack가 오기 전에 leave가 먼저 와도 홀드가 누수되지 않는다', () => {
    const { pool } = setup(1);
    const ed = {};
    pool.updatePageDemand(ed, 0, 'visible');
    const lease = pool.acquireCanvasLease(ed, 0, 'gl');
    if (lease.backend !== 'gl') throw new Error('테스트 전제 위반');

    pool.leave(ed, 0); // 엔트리는 사라지지만 lease는 유령 홀드로 남는다
    expect(pool.debugHoldCount()).toBe(1);

    pool.cancelReservation(lease.token, 'left-before-ack');
    expect(pool.debugHoldCount()).toBe(0);
  });

  it('gl-dead ack는 즉시 releasing으로 전이하고 실패를 기록하되, 슬롯 반납은 released ack까지 미룬다', () => {
    const { pool } = setup(1);
    const ed = {};
    pool.updatePageDemand(ed, 0, 'visible');
    const lease = pool.acquireCanvasLease(ed, 0, 'gl');
    if (lease.backend !== 'gl') throw new Error('테스트 전제 위반');

    pool.ackAttached(lease.token, 'gl-dead');
    expect(pool.debugHoldCount()).toBe(1); // releasing으로 계상 유지
    expect(pool.backendOf(ed, 0)).toBe('gl'); // 정책은 유지, 실제 처분만 대기

    pool.ackReleased(lease.token);
    expect(pool.debugHoldCount()).toBe(0);
  });

  it('gl 배정에 cpu가 실제로 붙으면 슬롯을 즉시 반납하고 실패를 기록한다', () => {
    const { pool, changes } = setup(1);
    const ed = {};
    expect(pool.updatePageDemand(ed, 0, 'visible')).toBe('gl');
    const lease = pool.acquireCanvasLease(ed, 0, 'gl');
    if (lease.backend !== 'gl') throw new Error('테스트 전제 위반');
    expect(pool.updatePageDemand(ed, 1, 'visible')).toBe('cpu'); // 예산 소진

    changes.length = 0;
    pool.ackAttached(lease.token, 'cpu');
    expect(pool.backendOf(ed, 0)).toBe('cpu');
    // 슬롯이 즉시 비고, 같은 rebalance 안에서 page 1이 곧바로(짧은 backoff 없이) 승격된다.
    expect(changes.some(([e, p, b]) => e === ed && p === 1 && b === 'gl')).toBe(true);
    expect(pool.debugHoldCount()).toBe(1); // page 0의 슬롯은 page 1로 즉시 이관됐다
  });

  it('직전에 실패한 페이지는 짧은 backoff 동안 곧바로 슬롯을 재획득하지 못한다', () => {
    let clock = 0;
    const { pool, timers } = setup(1, () => clock);
    const ed = {};
    pool.updatePageDemand(ed, 0, 'visible');
    const lease = pool.acquireCanvasLease(ed, 0, 'gl');
    if (lease.backend !== 'gl') throw new Error('테스트 전제 위반');

    pool.ackAttached(lease.token, 'cpu');
    expect(pool.backendOf(ed, 0)).toBe('cpu');

    clock = 2000;
    fireTimers(timers);
    expect(pool.backendOf(ed, 0)).toBe('gl');
  });

  it('연속 3회 실패 후 30초 쿨다운이 걸리고 leave/재진입에도 유지되며, 만료 시 자가 웨이크로 해제된다', () => {
    let clock = 0;
    const { pool, changes, timers } = setup(2, () => clock);
    const ed = {};
    const advance = (ms: number) => {
      clock += ms;
    };

    failAndRewin(pool, changes, timers, advance, ed, 0, 'visible');
    failAndRewin(pool, changes, timers, advance, ed, 0, 'visible');
    pool.ackAttached(claimLease(pool, changes, ed, 0, 'visible'), 'cpu'); // 3번째 실패 — 쿨다운 진입
    expect(pool.backendOf(ed, 0)).toBe('cpu');

    pool.leave(ed, 0);
    expect(pool.updatePageDemand(ed, 0, 'visible')).toBe('cpu'); // 실패 이력은 leave를 넘어 보존된다

    clock = 40_000;
    fireTimers(timers);
    expect(pool.backendOf(ed, 0)).toBe('gl');
  });

  it('committed gl present는 연속 실패 이력을 리셋한다', () => {
    let clock = 0;
    const { pool, changes, timers } = setup(2, () => clock);
    const ed = {};
    const advance = (ms: number) => {
      clock += ms;
    };

    failAndRewin(pool, changes, timers, advance, ed, 0, 'visible');
    failAndRewin(pool, changes, timers, advance, ed, 0, 'visible');
    expect(pool.backendOf(ed, 0)).toBe('gl'); // 아직 2회 — 3회 문턱 미도달

    const success = pool.acquireCanvasLease(ed, 0, 'gl');
    if (success.backend !== 'gl') throw new Error('테스트 전제 위반');
    pool.ackAttached(success.token, 'gl');
    pool.notePresent(ed, 0, success.token); // committed present — 실패 이력 리셋

    failAndRewin(pool, changes, timers, advance, ed, 0, 'visible');
    failAndRewin(pool, changes, timers, advance, ed, 0, 'visible');
    expect(pool.backendOf(ed, 0)).toBe('gl'); // 리셋되지 않았다면 누적 4회로 이미 차단됐을 것

    pool.ackAttached(claimLease(pool, changes, ed, 0, 'visible'), 'cpu'); // 리셋 이후 3회째
    expect(pool.backendOf(ed, 0)).toBe('cpu');
  });

  it('불일치 없는 정상 ack(gl→gl)는 아무 부작용 없이 live 상태로 전이한다', () => {
    const { pool } = setup(1);
    const ed = {};
    pool.updatePageDemand(ed, 0, 'visible');
    const lease = pool.acquireCanvasLease(ed, 0, 'gl');
    if (lease.backend !== 'gl') throw new Error('테스트 전제 위반');

    pool.ackAttached(lease.token, 'gl');
    expect(pool.backendOf(ed, 0)).toBe('gl');
    expect(pool.debugHoldCount()).toBe(1);
    pool.notePresent(ed, 0, lease.token);
    expect(pool.backendOf(ed, 0)).toBe('gl');
  });

  it('forget은 실패 이력을 지우고 leave는 보존한다', () => {
    let clock = 0;
    const { pool, changes, timers } = setup(2, () => clock);
    const forgetful = {};
    const lingering = {};
    const advance = (ms: number) => {
      clock += ms;
    };

    failAndRewin(pool, changes, timers, advance, forgetful, 0, 'visible');
    failAndRewin(pool, changes, timers, advance, forgetful, 0, 'visible');
    pool.ackAttached(claimLease(pool, changes, forgetful, 0, 'visible'), 'cpu'); // 3회 — 차단됨
    expect(pool.backendOf(forgetful, 0)).toBe('cpu');
    pool.forget(forgetful, 0);
    expect(pool.updatePageDemand(forgetful, 0, 'visible')).toBe('gl'); // 이력이 지워져 즉시 gl

    failAndRewin(pool, changes, timers, advance, lingering, 0, 'visible');
    failAndRewin(pool, changes, timers, advance, lingering, 0, 'visible');
    pool.ackAttached(claimLease(pool, changes, lingering, 0, 'visible'), 'cpu'); // 3회 — 차단됨
    pool.leave(lingering, 0);
    expect(pool.updatePageDemand(lingering, 0, 'visible')).toBe('cpu'); // 이력이 보존되어 여전히 cpu
  });

  it('removeEditor 이후에도 lease는 유지되다가 released ack가 와야 정확히 1회 해제된다', () => {
    const { pool } = setup(1);
    const a = {};
    const b = {};
    const leaseA = expectGlLease(pool, a, 0, 'visible');
    expect(pool.updatePageDemand(b, 0, 'visible')).toBe('cpu');

    pool.removeEditor(a);
    expect(pool.backendOf(b, 0)).toBe('cpu'); // a의 lease가 아직 살아있어 승격 안 됨
    expect(pool.debugHoldCount()).toBe(1);

    pool.ackReleased(leaseA);
    expect(pool.backendOf(b, 0)).toBe('gl');
    expect(pool.debugHoldCount()).toBe(1);

    pool.ackReleased(leaseA); // 중복 ack — 무시(orphan hold는 정확히 1회만 소비됨)
    expect(pool.debugHoldCount()).toBe(1);
  });

  it('간접적으로 승격되는 페이지의 콜백에는 즉시 사용 가능한 acquireHint가 함께 온다', () => {
    const { pool, changes } = setup(1);
    const a = {};
    const b = {};
    const leaseA = expectGlLease(pool, a, 0, 'visible');
    expect(pool.updatePageDemand(b, 0, 'overscan')).toBe('cpu'); // 예산 소진으로 대기

    pool.leave(a, 0);
    changes.length = 0;
    pool.ackReleased(leaseA);

    expect(changes).toHaveLength(1);
    const [editorKey, page, backend, hint] = changes[0];
    expect(editorKey).toBe(b);
    expect(page).toBe(0);
    expect(backend).toBe('gl');
    expect(hint).toBeTypeOf('number');
    if (hint === undefined) throw new Error('acquireHint가 없다');

    pool.ackAttached(hint, 'gl');
    expect(pool.backendOf(b, 0)).toBe('gl');
    expect(pool.debugHoldCount()).toBe(1);
  });

  it('콜백 안에서 풀 메서드를 다시 불러도 재진입 없이 수렴한다', () => {
    let calls = 0;
    const pool: GlContextPool = new GlContextPool(1, (e, p, b, hint) => {
      calls += 1;
      if (b === 'gl' && hint !== undefined) pool.noteGlFailure(e, p, hint);
    });
    const a = {};
    const b = {};
    const leaseA = expectGlLease(pool, a, 0, 'visible');
    expect(pool.updatePageDemand(b, 0, 'visible')).toBe('cpu');

    pool.leave(a, 0);
    pool.ackReleased(leaseA); // b 승격 → 콜백이 noteGlFailure로 재진입

    expect(calls).toBeGreaterThan(0);
    expect(calls).toBeLessThan(10);
    expect(pool.backendOf(b, 0)).toBe('gl'); // 실패 1회로는 보유자를 쫓아내지 않는다(3회 문턱)
  });

  it('포커스된 에디터의 페이지를 포커스 없는 visible 페이지보다 우대한다', () => {
    const { pool } = setup(2);
    const a = {};
    const b = {};
    pool.setFocus(b);
    expectGlLease(pool, a, 0, 'visible');
    expectGlLease(pool, a, 1, 'visible');
    // focused overscan(tier 1) > unfocused visible(tier 2): 슬롯이 다 차 정책 배정은 cpu로 유보되지만
    // (원장 오염 방지), b가 a:1을 눌러 강등시키는 것으로 우선순위가 반영됐음을 보인다.
    expect(pool.updatePageDemand(b, 0, 'overscan')).toBe('cpu');
    expect(pool.backendOf(a, 1)).toBe('cpu');
    expect(pool.backendOf(a, 0)).toBe('gl');
  });

  it('압박이 없으면 focus를 해제해도 강등하지 않는다', () => {
    const { pool, changes } = setup(2);
    const a = {};
    pool.setFocus(a);
    expectGlLease(pool, a, 0, 'visible');
    expectGlLease(pool, a, 1, 'visible');

    changes.length = 0;
    pool.clearFocus(a);
    expect(changes).toEqual([]);
    expect(pool.backendOf(a, 0)).toBe('gl');
    expect(pool.backendOf(a, 1)).toBe('gl');
  });

  it('포커스되지 않은 editorKey에 clearFocus를 호출해도 아무 일도 일어나지 않는다', () => {
    const { pool, changes } = setup(2);
    const a = {};
    const b = {};
    pool.setFocus(a);
    expectGlLease(pool, a, 0, 'visible');

    changes.length = 0;
    pool.clearFocus(b); // b는 애초에 focus를 가진 적이 없다 — no-op
    expect(changes).toEqual([]);
    expect(pool.backendOf(a, 0)).toBe('gl');
  });

  it('disabled 풀은 항상 cpu만 배정하고 lease를 만들지 않는다', () => {
    const pool = new GlContextPool(
      2,
      () => {
        throw new Error('disabled 풀은 콜백을 발생시키지 않아야 한다');
      },
      undefined,
      undefined,
      true,
    );
    const ed = {};
    expect(pool.updatePageDemand(ed, 0, 'visible')).toBe('cpu');
    expect(pool.acquireCanvasLease(ed, 0, 'gl')).toEqual({ backend: 'cpu' });
    expect(pool.debugHoldCount()).toBe(0);
  });
});
