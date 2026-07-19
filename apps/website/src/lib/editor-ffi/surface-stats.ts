// 프로덕션 1회성 계측(모바일 Safari, 다페이지 문서의 WebGL 컨텍스트 churn·blank 진단용).
// localStorage 'typie:surface-stats' 값이 '1'일 때만 활성화되며(모듈 초기화 시 1회 판독),
// 비활성 시 모든 훅은 boolean 체크 한 번으로 즉시 반환하는 no-op이다(프로덕션 영향 0).
//
// 출력 규율: heartbeat는 2000ms마다 카운터가 변했을 때만 compact 1줄(console.warn). 드문 사건
// (force-loss, failedParked, vvResize)은 즉시 로깅. 프레임 단위 로깅은 없다. blank 샘플러는
// 1000ms마다 등록 페이지를 순회해 새 blank-suspect만 1줄 로깅한다.
//
// 크래시 포렌식: 매 heartbeat가 compact 스냅샷을 localStorage 'typie:surface-stats:last'에 덮어쓴다
// (탭이 다음 heartbeat 전에 죽어도 마지막 상태가 남는다). 모듈 초기화 시 그 키가 있으면 직전 세션의
// 마지막 스냅샷(크래시 직전일 수 있음)을 한 번 경고하고 삭제한다.

import { GL_POOL_BUDGET, glContextPool } from './gl-context-pool';

const LAST_SNAPSHOT_KEY = 'typie:surface-stats:last';

const readFlag = (): boolean => {
  try {
    return typeof localStorage !== 'undefined' && localStorage.getItem('typie:surface-stats') === '1';
  } catch {
    return false;
  }
};

export const statsEnabled = readFlag();

const nowMs = (): number => (typeof performance === 'undefined' ? Date.now() : performance.now());
const timeStr = (): string => new Date().toISOString().slice(11, 23);

// ── blank 샘플러 등록 계약 ──────────────────────────────────────────────────
export type PageStatsReg = {
  page: number;
  wrapper: HTMLElement;
  isAttached: () => boolean;
  debug: () => { live: unknown; pending: unknown; wantsLive: boolean; timedOutOnce: boolean };
  poolBackend: () => string | undefined;
  ioSnapshot: () => { seeded: number; msSinceLastBuild: number; msSinceLastReconcile: number };
};

type PageRecord = { reg: PageStatsReg; blankSuspect: boolean };

// ── 카운터(활성 시에만 의미 있음) ───────────────────────────────────────────
let glCreate = 0;
let glDispose = 0;
let aliveNow = 0;
let alivePeak = 0;
let disposeWasLost = 0;
let unexpectedLost = 0;
let acquireGl = 0;
let acquireCpuFallback = 0;
let reconcileCount = 0;
let recycleHit = 0;
let recycleMiss = 0;
let recycleEvict = 0;
let finishSwapMaxMs = 0;
let seededGapCount = 0;
let seededGapMaxMs = 0;
let seededGapLastMs = 0;

const createRing: number[] = [];
const buildCounts = new Map<string, number>();
const mgrEvents = new Map<string, number>();
// 캔버스 신원 기반 dedup — 재활용 캔버스의 재-attach가 새 컨텍스트 생성으로 오계상되지 않게 한다.
const seenContexts = new WeakSet<object>();
const pages = new Set<PageRecord>();

let dirty = false;

// eslint-disable-next-line @typescript-eslint/no-empty-function -- 비활성 시 반환하는 no-op 등록 해제자
const noop = (): void => {};

const bump = (map: Map<string, number>, key: string): void => {
  map.set(key, (map.get(key) ?? 0) + 1);
};

const recentCreateRate = (windowMs: number): number => {
  const cutoff = nowMs() - windowMs;
  let n = 0;
  for (let i = createRing.length - 1; i >= 0; i--) {
    if (createRing[i] >= cutoff) n += 1;
    else break;
  }
  return n;
};

const snapshot = (): Record<string, unknown> => ({
  t: timeStr(),
  gl: {
    create: glCreate,
    dispose: glDispose,
    alive: aliveNow,
    peak: alivePeak,
    wasLost: disposeWasLost,
    unexpLost: unexpectedLost,
    rate2s: recentCreateRate(2000),
  },
  pool: { acqGl: acquireGl, acqCpuFb: acquireCpuFallback, hold: glContextPool.debugHoldCount(), budget: GL_POOL_BUDGET },
  io: {
    build: Object.fromEntries(buildCounts),
    reconcile: reconcileCount,
    seededGap: { n: seededGapCount, maxMs: Math.round(seededGapMaxMs), lastMs: Math.round(seededGapLastMs) },
  },
  mgr: Object.fromEntries(mgrEvents),
  recycle: { hit: recycleHit, miss: recycleMiss, evict: recycleEvict, finishMaxMs: Math.round(finishSwapMaxMs) },
  pages: pages.size,
});

const heartbeat = (): void => {
  if (!dirty) return;
  dirty = false;
  const snap = snapshot();
  console.warn('[surface-stats]', JSON.stringify(snap));
  try {
    localStorage.setItem(LAST_SNAPSHOT_KEY, JSON.stringify(snap));
  } catch {
    // localStorage 쓰기 실패는 계측 자체를 방해하지 않는다.
  }
};

const sampleBlank = (): void => {
  const vh = typeof window === 'undefined' ? 0 : window.innerHeight;
  for (const record of pages) {
    const { reg } = record;
    const rect = reg.wrapper.getBoundingClientRect();
    const intersects = rect.height > 0 && rect.bottom > 0 && rect.top < vh;
    const suspect = intersects && !reg.isAttached();
    if (suspect && !record.blankSuspect) {
      record.blankSuspect = true;
      const d = reg.debug();
      const io = reg.ioSnapshot();
      console.warn(
        '[surface-stats] BLANK-SUSPECT',
        JSON.stringify({
          page: reg.page,
          wantsLive: d.wantsLive,
          hasLive: !!d.live,
          hasPending: !!d.pending,
          timedOutOnce: d.timedOutOnce,
          poolBackend: reg.poolBackend() ?? null,
          seeded: io.seeded,
          msSinceLastBuild: Math.round(io.msSinceLastBuild),
          msSinceLastReconcile: Math.round(io.msSinceLastReconcile),
        }),
      );
    } else if (!suspect && record.blankSuspect) {
      record.blankSuspect = false;
    }
  }
};

// 모듈 초기화: 직전 세션의 마지막 스냅샷(크래시 직전일 수 있음)을 한 번 surface하고 삭제한다.
if (typeof localStorage !== 'undefined') {
  try {
    const last = localStorage.getItem(LAST_SNAPSHOT_KEY);
    if (last !== null) {
      console.warn('[surface-stats] 이전 세션 마지막 스냅샷(크래시 직전일 수 있음):', last);
      localStorage.removeItem(LAST_SNAPSHOT_KEY);
    }
  } catch {
    // 무시.
  }
}

if (statsEnabled && typeof window !== 'undefined') {
  setInterval(heartbeat, 2000);
  setInterval(sampleBlank, 1000);
  console.warn('[surface-stats] enabled (budget=' + GL_POOL_BUDGET + ')');
}

export const surfaceStats = {
  // GL 컨텍스트 생성(GL attach 성공 시). canvas 신원으로 dedup — 재활용 재-attach는 재계상하지 않는다.
  glCreate(canvas: object): void {
    if (!statsEnabled) return;
    if (seenContexts.has(canvas)) return;
    seenContexts.add(canvas);
    glCreate += 1;
    aliveNow += 1;
    if (aliveNow > alivePeak) alivePeak = aliveNow;
    createRing.push(nowMs());
    if (createRing.length > 100) createRing.shift();
    dirty = true;
  },
  // GL 컨텍스트 처분(loseContext 또는 재활용 축출). wasLost=디스포즈 시점에 이미 로스였는지
  // ("already lost" 스팸의 출처 판별).
  glDispose(canvas: object, wasLost: boolean): void {
    if (!statsEnabled) return;
    if (!seenContexts.delete(canvas)) return;
    glDispose += 1;
    aliveNow -= 1;
    if (wasLost) disposeWasLost += 1;
    dirty = true;
  },
  // 우리가 방금 처분하지 않은 캔버스의 webglcontextlost(=force-loss 감지). 즉시 로깅.
  unexpectedLost(page: number): void {
    if (!statsEnabled) return;
    unexpectedLost += 1;
    dirty = true;
    console.warn('[surface-stats] unexpectedLost page=' + page + ' (force-loss suspect) at ' + timeStr());
  },
  // 풀 어댑터 wrap: gl 요청이 실제 gl lease를 받았는지 / 예산 부족으로 cpu로 떨어졌는지.
  acquireGl(): void {
    if (!statsEnabled) return;
    acquireGl += 1;
    dirty = true;
  },
  acquireCpuFallback(): void {
    if (!statsEnabled) return;
    acquireCpuFallback += 1;
    dirty = true;
  },
  // 매니저 이벤트(ManagerEffects.note). 'mount:<cause>' | 'park' | 'swapTimeout1' | 'swapTimeout2'
  // | 'finishSwap:<ms>' | 'resume'. swapTimeout2(failedParked)는 즉시 로깅한다.
  managerEvent(page: number, event: string): void {
    if (!statsEnabled) return;
    dirty = true;
    if (event.startsWith('finishSwap')) {
      const ms = Number(event.slice(event.indexOf(':') + 1)) || 0;
      if (ms > finishSwapMaxMs) finishSwapMaxMs = ms;
      bump(mgrEvents, 'finishSwap');
      return;
    }
    bump(mgrEvents, event);
    if (event === 'swapTimeout2') {
      console.warn('[surface-stats] swapTimeout2->failedParked page=' + page + ' at ' + timeStr());
    }
  },
  // IO 레이어: build cause별 카운트.
  build(cause: string): void {
    if (!statsEnabled) return;
    bump(buildCounts, cause);
    dirty = true;
  },
  // build→seeded>=3 소요 시간.
  seededGap(ms: number): void {
    if (!statsEnabled) return;
    seededGapCount += 1;
    seededGapLastMs = ms;
    if (ms > seededGapMaxMs) seededGapMaxMs = ms;
    dirty = true;
  },
  reconcile(): void {
    if (!statsEnabled) return;
    reconcileCount += 1;
    dirty = true;
  },
  // visualViewport 리사이즈: innerHeight vs visualViewport.height를 즉시 로깅(툴바 collapse 추적).
  visualViewportResize(innerHeight: number, vvHeight: number): void {
    if (!statsEnabled) return;
    dirty = true;
    console.warn('[surface-stats] vvResize innerH=' + innerHeight + ' vvH=' + Math.round(vvHeight) + ' at ' + timeStr());
  },
  recycleHit(): void {
    if (!statsEnabled) return;
    recycleHit += 1;
    dirty = true;
  },
  recycleMiss(): void {
    if (!statsEnabled) return;
    recycleMiss += 1;
    dirty = true;
  },
  recycleEvict(): void {
    if (!statsEnabled) return;
    recycleEvict += 1;
    dirty = true;
  },
  // blank 샘플러 등록. 반환값은 등록 해제 함수(teardown에서 호출).
  registerPage(reg: PageStatsReg): () => void {
    if (!statsEnabled) return noop;
    const record: PageRecord = { reg, blankSuspect: false };
    pages.add(record);
    return () => {
      pages.delete(record);
    };
  },
};
