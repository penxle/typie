import { describe, expect, it } from 'vitest';
import { GlContextPool } from './gl-context-pool';
import { createPageSurfaceManager, RESTORE_WATCHDOG_MS, SWAP_TIMEOUT_MS } from './page-surface-manager';
import type { AttachOutcome, LeaseToken, PageZone, SurfaceBackend } from './gl-context-pool';
import type { ManagerEffects, PoolPort, VisibilityState } from './page-surface-manager';

// 통합·property 수용 스위트: 실제 GlContextPool + 실제 createPageSurfaceManager를 연결한다.
// fake는 효과 경계(attach 결과 스크립트, defer 큐, 수동 타이머, DOM 카운터)에만 있다 — pool은
// 절대 fake로 대체하지 않는다(fake pool은 lease 레이스를 구조적으로 통과시킨다).

type ScriptedOutcome = AttachOutcome | 'cpu-oversized';

type FakeCanvas = {
  id: number;
  token: LeaseToken | undefined;
  requestedBackend: SurfaceBackend | undefined;
  outcome: ScriptedOutcome | undefined;
  isGl: boolean;
  disposedGl: boolean;
  disposedCpu: boolean;
  removed: boolean;
  listeners: number;
};

type TimerEntry = { id: number; at: number; ms: number; label: string; fn: () => void; cancelled: boolean };

type PageHandle = {
  editorKey: object;
  page: number;
  manager: ReturnType<typeof createPageSurfaceManager<FakeCanvas>>;
  canvases: FakeCanvas[];
  timers: TimerEntry[];
  deferred: (() => void)[];
  presented: (() => void)[];
  attachScript: ScriptedOutcome[];
  attachRequests: SurfaceBackend[];
  state: {
    attachedCount: number;
    pendingToken: LeaseToken | undefined;
    deferSync: boolean;
    lastZone: PageZone | undefined;
    lastVisibility: VisibilityState | undefined;
  };
};

const inView: VisibilityState = { inAcquire: true, inRelease: true, isVisible: true };
const outOfView: VisibilityState = { inAcquire: false, inRelease: false, isVisible: false };
const overscan: VisibilityState = { inAcquire: true, inRelease: true, isVisible: false };

function createHarness(budget: number, opts?: { deferSync?: boolean }) {
  let clock = 0;
  let timerId = 0;
  let canvasId = 0;
  let presentCounter = 0;
  const poolTimers: TimerEntry[] = [];
  const pagesByEditor = new Map<object, Map<number, PageHandle>>();
  const graveyard: PageHandle[] = [];
  const opLog: string[] = [];
  const lastPresentOrder = new Map<string, number>();
  const editorIds = new WeakMap<object, number>();
  let nextEditorId = 0;

  const keyFor = (editorKey: object, page: number) => {
    let id = editorIds.get(editorKey);
    if (id === undefined) {
      id = ++nextEditorId;
      editorIds.set(editorKey, id);
    }
    return `e${id}#${page}`;
  };
  const now = () => clock;

  const poolSchedule = (fn: () => void, ms: number) => {
    const entry: TimerEntry = { id: ++timerId, at: clock + ms, ms, label: 'pool-cooldown', fn, cancelled: false };
    poolTimers.push(entry);
    return () => {
      entry.cancelled = true;
    };
  };

  // recyclePage/removeEditorAbrupt는 (editorKey, page) 좌표를 재사용해 새 핸들을 만든다 — 그
  // 순간에도 옛(무덤행) 핸들이 여전히 살아있는 lease를 들고 있을 수 있으므로, 좌표만으로는 어느
  // 핸들이 어느 lease의 진짜 소유자인지 판별할 수 없다. 소유권을 핸들 신원(state 객체) 단위로
  // 별도 추적해 I6 검증이 좌표 재사용에 오판하지 않게 한다.
  const tokenOwner = new Map<LeaseToken, PageHandle['state']>();

  const pool = new GlContextPool(
    budget,
    (editorKey, page, backend, acquireHint) => {
      const target = pagesByEditor.get(editorKey)?.get(page);
      if (!target) return;
      target.state.pendingToken = acquireHint;
      if (acquireHint !== undefined) tokenOwner.set(acquireHint, target.state);
      target.manager.onPoolBackend(backend, acquireHint);
    },
    now,
    poolSchedule,
  );

  function makePoolPort(editorKey: object, page: number, state: PageHandle['state']): PoolPort {
    return {
      updateDemand: (zone) => pool.updatePageDemand(editorKey, page, zone),
      acquireLease: (requested) => {
        const result = pool.acquireCanvasLease(editorKey, page, requested);
        if (result.backend === 'gl') {
          state.pendingToken = result.token;
          tokenOwner.set(result.token, state);
        }
        return result;
      },
      ackAttached: (token, actual) => pool.ackAttached(token, actual),
      cancelReservation: (token, reason) => pool.cancelReservation(token, reason),
      beginRelease: (token) => pool.beginRelease(token),
      ackReleased: (token) => pool.ackReleased(token),
      notePresent: (token) => {
        pool.notePresent(editorKey, page, token);
        presentCounter += 1;
        lastPresentOrder.set(keyFor(editorKey, page), presentCounter);
      },
      noteGlFailure: (incident) => pool.noteGlFailure(editorKey, page, incident),
      noteBudgetFallback: () => pool.noteBudgetFallback(editorKey, page),
      backendOf: () => pool.backendOf(editorKey, page),
      leave: () => pool.leave(editorKey, page),
      forget: () => pool.forget(editorKey, page),
    };
  }

  function addPage(editorKey: object, page: number, deferSyncOverride?: boolean): PageHandle {
    let pages = pagesByEditor.get(editorKey);
    if (!pages) {
      pages = new Map();
      pagesByEditor.set(editorKey, pages);
    }

    const canvases: FakeCanvas[] = [];
    const timers: TimerEntry[] = [];
    const deferred: (() => void)[] = [];
    const presented: (() => void)[] = [];
    const attachScript: ScriptedOutcome[] = [];
    const attachRequests: SurfaceBackend[] = [];
    const state = {
      attachedCount: 0,
      pendingToken: undefined as LeaseToken | undefined,
      deferSync: deferSyncOverride ?? opts?.deferSync ?? false,
      lastZone: undefined as PageZone | undefined,
      lastVisibility: undefined as VisibilityState | undefined,
    };

    const effects: ManagerEffects<FakeCanvas> = {
      createCanvas: () => {
        const canvas: FakeCanvas = {
          id: ++canvasId,
          token: state.pendingToken,
          requestedBackend: undefined,
          outcome: undefined,
          isGl: false,
          disposedGl: false,
          disposedCpu: false,
          removed: false,
          listeners: 0,
        };
        state.pendingToken = undefined;
        canvases.push(canvas);
        return canvas;
      },
      // eslint-disable-next-line @typescript-eslint/no-empty-function -- styling isn't under test here
      styleCanvas: () => {},
      attach: (canvas, backend) => {
        attachRequests.push(backend);
        canvas.requestedBackend = backend;
        const scripted = attachScript.shift();
        let outcome: ScriptedOutcome = scripted ?? backend;
        if (backend === 'cpu' && (outcome === 'gl' || outcome === 'gl-dead')) {
          // cpu로 요청된 attach는 애초에 gl 컨텍스트를 시도하지 않으므로 gl 계열 결과가 나올 수
          // 없다 — 스크립트에 gl 계열이 큐잉돼 있어도 실제로는 cpu 성공으로 수렴한다.
          outcome = 'cpu';
        }
        canvas.outcome = outcome;
        canvas.isGl = outcome === 'gl';
        state.attachedCount += 1;
        return outcome;
      },
      detach: () => {
        state.attachedCount -= 1;
      },
      // eslint-disable-next-line @typescript-eslint/no-empty-function -- render scheduling isn't under test here
      requestRender: () => {},
      onPresented: (listener) => {
        presented.push(listener);
        return () => {
          const index = presented.indexOf(listener);
          if (index !== -1) presented.splice(index, 1);
        };
      },
      addContextListeners: (canvas) => {
        canvas.listeners += 1;
        return () => {
          canvas.listeners -= 1;
        };
      },
      disposeGlContext: (canvas) => {
        canvas.disposedGl = true;
      },
      releaseCpuBacking: (canvas) => {
        canvas.disposedCpu = true;
      },
      // eslint-disable-next-line @typescript-eslint/no-empty-function -- DOM append isn't under test here
      promote: () => {},
      removeNode: (canvas) => {
        canvas.removed = true;
      },
      schedule: (fn, ms) => {
        const label = ms === SWAP_TIMEOUT_MS ? 'swap-timeout' : ms === RESTORE_WATCHDOG_MS ? 'restore-watchdog' : `ms:${ms}`;
        const entry: TimerEntry = { id: ++timerId, at: clock + ms, ms, label, fn, cancelled: false };
        timers.push(entry);
        return () => {
          entry.cancelled = true;
        };
      },
      defer: (fn) => {
        if (state.deferSync) fn();
        else deferred.push(fn);
      },
      pool: makePoolPort(editorKey, page, state),
    };

    const manager = createPageSurfaceManager(effects);
    const handle: PageHandle = { editorKey, page, manager, canvases, timers, deferred, presented, attachScript, attachRequests, state };
    pages.set(page, handle);
    return handle;
  }

  function activePages(): PageHandle[] {
    return [...pagesByEditor.values()].flatMap((pages) => [...pages.values()]);
  }

  function knownPages(): PageHandle[] {
    return [...activePages(), ...graveyard];
  }

  function fireDueTimersOnce(): number {
    let fired = 0;
    for (const page of knownPages()) {
      const due = page.timers.filter((t) => !t.cancelled && t.at <= clock);
      for (const t of due) {
        t.cancelled = true;
        t.fn();
        fired += 1;
      }
    }
    const poolDue = poolTimers.filter((t) => !t.cancelled && t.at <= clock);
    for (const t of poolDue) {
      t.cancelled = true;
      t.fn();
      fired += 1;
    }
    return fired;
  }

  function flushAllDeferOnce(): number {
    let flushed = 0;
    for (const page of knownPages()) {
      if (page.state.deferSync) continue;
      const due = [...page.deferred];
      page.deferred.length = 0;
      for (const fn of due) {
        fn();
        flushed += 1;
      }
    }
    return flushed;
  }

  function isFullyQuiescent(): boolean {
    const anyDeferred = knownPages().some((p) => p.deferred.length > 0);
    const anyTimers = knownPages().some((p) => p.timers.some((t) => !t.cancelled)) || poolTimers.some((t) => !t.cancelled);
    return !anyDeferred && !anyTimers;
  }

  function assertInvariants(): void {
    // I1: 예산 불변식.
    expect(pool.debugHoldCount()).toBeLessThanOrEqual(budget);

    const snapshot = pool.debugLeaseSnapshot();
    // I6ⓓ: leaseId 중복 없음, 논리 카운트 예산 이하.
    const ids = new Set(snapshot.map((s) => s.leaseId));
    expect(ids.size).toBe(snapshot.length);
    expect(snapshot.length).toBeLessThanOrEqual(budget);

    let globalAttachedManagers = 0;
    let globalAttachedCountSum = 0;
    let globalUndisposedGlCanvases = 0;

    for (const page of knownPages()) {
      const debug = page.manager.debug();
      const notRemoved = page.canvases.filter((c) => !c.removed);
      const expectedSet = new Set<FakeCanvas>([debug.live, debug.pending].filter((c): c is FakeCanvas => c !== undefined));

      // I2: 페이지당 DOM 캔버스 ≤ 1(live) + ≤ 1(pending); 미처분 캔버스는 전부 live/pending 중 하나.
      expect(notRemoved.length).toBeLessThanOrEqual(2);
      for (const c of notRemoved) expect(expectedSet.has(c)).toBe(true);
      for (const c of expectedSet) expect(c.removed).toBe(false);

      // I3: wasmAttached ⇔ state ∈ {pending, live}; per-page attach 카운터는 0 또는 1.
      const wasmAttached = page.manager.isAttached();
      const shouldBeAttached = debug.live !== undefined || debug.pending !== undefined;
      expect(wasmAttached).toBe(shouldBeAttached);
      expect(page.state.attachedCount === 0 || page.state.attachedCount === 1).toBe(true);
      expect(page.state.attachedCount > 0).toBe(wasmAttached);
      globalAttachedManagers += wasmAttached ? 1 : 0;
      globalAttachedCountSum += page.state.attachedCount;

      // I4: 처분된 캔버스는 리스너 0; live/pending 캔버스는 리스너 1.
      for (const c of page.canvases) {
        if (c.removed) expect(c.listeners).toBe(0);
      }
      for (const c of expectedSet) expect(c.listeners).toBe(1);

      // I6ⓐⓑ: 페이지 단위로 hold ↔ 캔버스 대응을 검증한다. (editorKey, page) 좌표가 아니라
      // 핸들 신원(tokenOwner)으로 소유권을 판별한다 — recyclePage/removeEditorAbrupt가 좌표를
      // 재사용해도 무덤행 핸들의 잔존 lease를 새 핸들의 것으로 오판하지 않기 위함이다.
      const pageLeases = snapshot.filter((s) => tokenOwner.get(s.leaseId) === page.state);
      const glCanvases = notRemoved.filter((c) => c.isGl);
      globalUndisposedGlCanvases += glCanvases.length;
      for (const c of glCanvases) {
        expect(c.token).toBeDefined();
        expect(pageLeases.filter((l) => l.leaseId === c.token)).toHaveLength(1);
      }
      for (const lease of pageLeases) {
        const matches = glCanvases.filter((c) => c.token === lease.leaseId);
        if (lease.phase === 'live') {
          // ⓑ live hold는 정확히 하나의 미처분 GL 캔버스에 대응한다.
          expect(matches).toHaveLength(1);
        } else if (lease.phase === 'releasing') {
          // releasing 전이는 캔버스 처분과 동일 tick에 원자적으로 일어난다(disposeSlot이
          // removeNode 후 beginRelease를 호출) — 관찰 가능한 시점에 캔버스가 남아있지 않는다.
          expect(matches).toHaveLength(0);
        }
        // ⓒ reserved hold는 attach 시작 전이면 캔버스가 없어도 된다(0 또는 1 모두 허용).
      }
    }

    expect(globalAttachedCountSum).toBe(globalAttachedManagers);
    expect(globalUndisposedGlCanvases).toBeLessThanOrEqual(budget);

    // I5(조건부): defer/타이머가 모두 소진된 순간이면, orphan(reserved/releasing) hold와
    // pending 슬롯이 하나도 남아있지 않아야 한다.
    if (isFullyQuiescent()) {
      for (const lease of snapshot) expect(lease.phase).toBe('live');
      for (const page of knownPages()) expect(page.manager.debug().pending).toBeUndefined();
    }
  }

  const log = (entry: string) => {
    opLog.push(entry);
  };

  const h = {
    pool,
    budget,
    opLog,
    now: () => clock,
    addPage(editorKey: object, page: number, deferSyncOverride?: boolean): PageHandle {
      const handle = addPage(editorKey, page, deferSyncOverride);
      log(`addPage(${keyFor(editorKey, page)})`);
      assertInvariants();
      return handle;
    },
    reconcile(page: PageHandle, visibility: VisibilityState): void {
      log(`reconcile(${keyFor(page.editorKey, page.page)}, ${JSON.stringify(visibility)})`);
      page.state.lastZone = visibility.isVisible ? 'visible' : 'overscan';
      page.state.lastVisibility = visibility;
      page.manager.reconcile(visibility);
      assertInvariants();
    },
    setAttachScript(page: PageHandle, outcomes: ScriptedOutcome[]): void {
      page.attachScript.push(...outcomes);
      log(`setAttachScript(${keyFor(page.editorKey, page.page)}, ${JSON.stringify(outcomes)})`);
    },
    onPoolBackend(page: PageHandle, backend: SurfaceBackend, hint?: LeaseToken): void {
      log(`onPoolBackend(${keyFor(page.editorKey, page.page)}, ${backend}, ${hint ?? '-'})`);
      page.state.pendingToken = hint;
      if (hint !== undefined) tokenOwner.set(hint, page.state);
      page.manager.onPoolBackend(backend, hint);
      assertInvariants();
    },
    flushDefer(page: PageHandle): number {
      const due = [...page.deferred];
      page.deferred.length = 0;
      for (const fn of due) fn();
      log(`flushDefer(${keyFor(page.editorKey, page.page)}) x${due.length}`);
      assertInvariants();
      return due.length;
    },
    flushAllDefer(): void {
      log('flushAllDefer()');
      let iterations = 0;
      while (flushAllDeferOnce() > 0) {
        iterations += 1;
        if (iterations > 500) throw new Error('flushAllDefer()가 500회 반복 내에 수렴하지 않음');
      }
      assertInvariants();
    },
    fireAllTimers(page: PageHandle): number {
      const due = page.timers.filter((t) => !t.cancelled);
      for (const t of due) t.cancelled = true;
      for (const t of due) t.fn();
      log(`fireAllTimers(${keyFor(page.editorKey, page.page)}) x${due.length}`);
      assertInvariants();
      return due.length;
    },
    fireTimerLabel(page: PageHandle, label: string): boolean {
      const timer = page.timers.findLast((t) => t.label === label && !t.cancelled);
      if (!timer) return false;
      timer.cancelled = true;
      timer.fn();
      log(`fireTimerLabel(${keyFor(page.editorKey, page.page)}, ${label})`);
      assertInvariants();
      return true;
    },
    firePoolTimers(): number {
      const due = poolTimers.filter((t) => !t.cancelled);
      for (const t of due) t.cancelled = true;
      for (const t of due) t.fn();
      log(`firePoolTimers() x${due.length}`);
      assertInvariants();
      return due.length;
    },
    firePresent(page: PageHandle): void {
      const due = [...page.presented];
      page.presented.length = 0;
      for (const fn of due) fn();
      log(`firePresent(${keyFor(page.editorKey, page.page)}) x${due.length}`);
      assertInvariants();
    },
    advanceClock(ms: number): void {
      clock += ms;
      log(`advanceClock(${ms}) -> ${clock}`);
      assertInvariants();
    },
    onContextLost(page: PageHandle): void {
      log(`onContextLost(${keyFor(page.editorKey, page.page)})`);
      page.manager.onContextLost();
      assertInvariants();
    },
    onContextRestored(page: PageHandle): void {
      log(`onContextRestored(${keyFor(page.editorKey, page.page)})`);
      page.manager.onContextRestored();
      assertInvariants();
    },
    setFocus(editorKey: object): void {
      log(`setFocus`);
      pool.setFocus(editorKey);
      assertInvariants();
    },
    clearFocus(editorKey: object): void {
      log(`clearFocus`);
      pool.clearFocus(editorKey);
      assertInvariants();
    },
    destroyPage(page: PageHandle): void {
      log(`destroyPage(${keyFor(page.editorKey, page.page)})`);
      page.manager.destroy();
      assertInvariants();
    },
    recyclePage(page: PageHandle, deferSyncOverride?: boolean): PageHandle {
      page.manager.destroy();
      pagesByEditor.get(page.editorKey)?.delete(page.page);
      graveyard.push(page);
      const fresh = addPage(page.editorKey, page.page, deferSyncOverride ?? page.state.deferSync);
      log(`recyclePage(${keyFor(page.editorKey, page.page)})`);
      assertInvariants();
      return fresh;
    },
    removeEditorAbrupt(editorKey: object): void {
      const pages = [...(pagesByEditor.get(editorKey)?.values() ?? [])];
      pool.removeEditor(editorKey);
      pagesByEditor.delete(editorKey);
      graveyard.push(...pages);
      log(`removeEditorAbrupt(${pages.map((p) => p.page).join(',')})`);
      assertInvariants();
    },
    quiesce(maxIterations = 500): void {
      log('quiesce()');
      for (let i = 0; i < maxIterations; i++) {
        const flushed = flushAllDeferOnce();
        const fired = fireDueTimersOnce();
        if (flushed === 0 && fired === 0) {
          assertInvariants();
          return;
        }
      }
      throw new Error(`quiesce()가 ${maxIterations}회 반복 내에 수렴하지 않음(thrash 의심)`);
    },
    isFullyQuiescent,
    activePages,
    knownPages,
    lastPresentOrder,
    keyFor,
    assertInvariants,
  };

  return h;
}

type Harness = ReturnType<typeof createHarness>;

// 정상 경로를 끝까지 밀어붙여(mount+ack+present) live gl 상태로 만든다. 테스트 전제가 깨지면
// (스크립트가 gl이 아니거나 예산이 없으면) 원인을 숨기지 않고 바로 실패시킨다.
function mountLiveGl(h: Harness, page: PageHandle): LeaseToken {
  h.setAttachScript(page, ['gl']);
  h.reconcile(page, inView);
  const token = page.canvases.at(-1)?.token;
  if (token === undefined) throw new Error('테스트 전제 위반: gl 토큰 없이 마운트됨');
  h.flushDefer(page);
  h.firePresent(page);
  if (page.manager.debug().live?.token !== token) throw new Error('테스트 전제 위반: live 전환 실패');
  return token;
}

// Step 2e: 순수 참조 정책 모델(스펙 §3.4) — 안전성(I1~I7)과 별도로 "누가 당첨돼야 하는가"를
// 판정한다. gl-context-pool.ts의 #tier/#rebalance 정렬 로직과는 독립적으로 브리프의 서열
// 서술만 보고 다시 구현한 것 — 구현을 그대로 베끼면 같은 버그를 공유해 판별력이 없어진다.
type PolicyEntry = { editorKey: object; page: number; zone: PageZone; isHolder: boolean; lastPresent: number };

function expectedWinners(entries: PolicyEntry[], focused: object | null, budget: number): Set<PolicyEntry> {
  const tier = (e: PolicyEntry): number => {
    const isFocused = e.editorKey === focused;
    if (isFocused) return e.zone === 'visible' ? 0 : 1;
    return e.zone === 'visible' ? 2 : 3;
  };
  const sorted = entries.toSorted((a, b) => tier(a) - tier(b) || Number(b.isHolder) - Number(a.isHolder) || b.lastPresent - a.lastPresent);
  return new Set(sorted.slice(0, budget));
}

function currentDesiredGlSet(h: Harness): Set<string> {
  const set = new Set<string>();
  for (const page of h.activePages()) {
    if (h.pool.backendOf(page.editorKey, page.page) === 'gl') set.add(h.keyFor(page.editorKey, page.page));
  }
  return set;
}

// 원장(backendOf)과 관리자의 realized backend(live 캔버스의 isGl)가 일치하는지 검증한다 —
// silent 오염(원장 gl인데 realized cpu)을 직접 잡는다. gl 실패/폴백이 없는 순수 정책
// 시나리오에서만 무조건 성립하므로 Step 2e 완전 quiescence 체크포인트에서만 호출한다.
function assertRealizedMatchesLedger(h: Harness): void {
  for (const page of h.activePages()) {
    const ledger = h.pool.backendOf(page.editorKey, page.page);
    const live = page.manager.debug().live;
    if (ledger === undefined || live === undefined) continue;
    expect(live.isGl).toBe(ledger === 'gl');
  }
}

function modelWinnersFor(h: Harness, focused: object | null, budget: number): Set<string> {
  const entries: PolicyEntry[] = [];
  for (const page of h.activePages()) {
    if (!page.manager.debug().wantsLive) continue; // 원치 않는 페이지는 정책 경쟁에 참여하지 않는다
    const zone = page.state.lastZone;
    if (!zone) continue;
    entries.push({
      editorKey: page.editorKey,
      page: page.page,
      zone,
      isHolder: h.pool.backendOf(page.editorKey, page.page) === 'gl',
      lastPresent: h.lastPresentOrder.get(h.keyFor(page.editorKey, page.page)) ?? 0,
    });
  }
  const winners = expectedWinners(entries, focused, budget);
  return new Set([...winners].map((e) => h.keyFor(e.editorKey, e.page)));
}

// 시드 고정 PRNG(mulberry32) — Step 2e 랜덤 비교와 Step 3 property 테스트가 공유한다.
function mulberry32(seed: number): () => number {
  let a = seed;
  return () => {
    // 32비트 정수 오버플로 랩어라운드(ToInt32)에 의존하는 비트 뒤섞기 — Math.trunc는 이
    // 랩어라운드를 재현하지 못하므로 `| 0` 관용구를 그대로 쓴다.
    // eslint-disable-next-line unicorn/prefer-math-trunc
    a = (a + 0x6d_2b_79_f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t ^= t + Math.imul(t ^ (t >>> 7), 61 | t);
    return ((t ^ (t >>> 14)) >>> 0) / 4_294_967_296;
  };
}

describe('page-surface-integration', () => {
  describe('Step 1: 하니스 스모크 — 기본 라이프사이클에서 I1~I7 유지', () => {
    it('mount → ack → present → park 전 구간에서 불변식이 깨지지 않는다', () => {
      const h = createHarness(2);
      const ed = {};
      const page = h.addPage(ed, 0);
      const token = mountLiveGl(h, page);
      expect(h.pool.backendOf(ed, 0)).toBe('gl');
      expect(h.pool.debugHoldCount()).toBe(1);

      h.reconcile(page, outOfView);
      h.quiesce();
      expect(h.pool.debugHoldCount()).toBe(0);
      expect(page.manager.debug().live).toBeUndefined();
      void token;
    });

    it('두 페이지 × cpu 오버플로 대안 경로도 불변식을 유지한다', () => {
      const h = createHarness(1);
      const ed = {};
      const p0 = h.addPage(ed, 0);
      const p1 = h.addPage(ed, 1);
      mountLiveGl(h, p0);
      h.setAttachScript(p1, ['cpu']);
      h.reconcile(p1, inView);
      h.flushDefer(p1);
      h.firePresent(p1);
      expect(h.pool.backendOf(ed, 1)).toBe('cpu');
      h.quiesce();
    });
  });

  describe('Step 2: 명명된 회귀 케이스', () => {
    it('R-A: GL→GL 재마운트는 중첩 구간 동안 hold 2, 구 release ack 후 hold 1로 수렴한다', () => {
      const h = createHarness(2);
      const ed = {};
      const page = h.addPage(ed, 0);
      const token1 = mountLiveGl(h, page);
      expect(h.pool.debugHoldCount()).toBe(1);

      const second = h.pool.acquireCanvasLease(ed, 0, 'gl');
      if (second.backend !== 'gl') throw new Error('테스트 전제 위반: 2번째 lease 확보 실패');
      h.setAttachScript(page, ['gl']);
      h.onPoolBackend(page, 'gl', second.token);

      expect(h.pool.debugHoldCount()).toBe(2);
      const overlapping = h.pool.debugLeaseSnapshot().filter((s) => s.editorKey === ed && s.page === 0);
      expect(new Set(overlapping.map((s) => s.leaseId))).toEqual(new Set([token1, second.token]));

      h.flushDefer(page); // 신 lease ack(gl→live)
      h.firePresent(page); // 커밋: 구 live 처분(beginRelease) — released ack는 아직 defer 중
      expect(h.pool.debugHoldCount()).toBe(2);

      h.flushDefer(page); // 구 release ack flush
      expect(h.pool.debugHoldCount()).toBe(1);
      expect(page.manager.debug().live?.token).toBe(second.token);
      expect(h.pool.backendOf(ed, 0)).toBe('gl');
    });

    it.each([
      ['cpu로 실착', 'cpu' as const],
      ['gl로 실착', 'gl' as const],
    ])('R-B(%s): ack flush 전 park돼도 hold가 새지 않는다', (_label, outcome) => {
      const h = createHarness(1);
      const ed = {};
      const page = h.addPage(ed, 0);
      h.setAttachScript(page, [outcome]);
      h.reconcile(page, inView);
      expect(h.pool.debugHoldCount()).toBe(1); // reserved, ack 미flush

      h.reconcile(page, outOfView); // ack flush 전에 park
      expect(h.pool.debugHoldCount()).toBe(0); // cancelReservation은 항상 동기

      h.flushDefer(page); // 이미 큐에 있던 ackAttached도 안전하게 무시된다
      h.quiesce();
      expect(h.pool.debugHoldCount()).toBe(0);
      expect(h.pool.debugLeaseSnapshot()).toHaveLength(0);
    });

    it('R-C: swap timeout 폴백은 반드시 cpu로 요청하고, 구 live를 유지하다가 폴백 commit으로 교체한다', () => {
      const h = createHarness(2);
      const ed = {};
      const page = h.addPage(ed, 0);
      mountLiveGl(h, page);
      const firstLive = page.manager.debug().live;

      const second = h.pool.acquireCanvasLease(ed, 0, 'gl');
      if (second.backend !== 'gl') throw new Error('테스트 전제 위반');
      h.setAttachScript(page, ['gl']);
      h.onPoolBackend(page, 'gl', second.token); // 신 pending — ack는 flush하지 않아 미커밋 유지

      page.attachRequests.length = 0;
      h.fireTimerLabel(page, 'swap-timeout');

      expect(page.attachRequests).toEqual(['cpu']); // 폴백은 gl 재요청이 아니라 cpu여야 한다
      expect(page.manager.debug().live).toBe(firstLive); // 구 live는 시각적으로 유지
      expect(h.pool.backendOf(ed, 0)).toBe('gl'); // enter() 재호출 없음(풀 재배정 없음)

      h.quiesce();
      h.firePresent(page); // 폴백 commit
      h.quiesce();
      expect(page.manager.debug().live).not.toBe(firstLive);
      expect(page.manager.isAttached()).toBe(true);
      expect(page.manager.debug().live?.outcome).toBe('cpu');
    });

    it('R-D: 로스 워치독은 restored 없이 1회 재마운트로 수렴하고, restored 선착 시 워치독을 취소한다', () => {
      const h = createHarness(2);
      const ed = {};
      const page = h.addPage(ed, 0);
      mountLiveGl(h, page);
      const firstLive = page.manager.debug().live;

      h.onContextLost(page);
      expect(page.timers.some((t) => t.label === 'restore-watchdog' && !t.cancelled)).toBe(true);

      h.setAttachScript(page, ['gl']);
      h.fireTimerLabel(page, 'restore-watchdog');
      expect(page.manager.debug().live).toBe(firstLive); // 아직 커밋 전 — 구 live 유지
      expect(page.manager.debug().pending).toBeDefined();

      h.quiesce();
      h.firePresent(page);
      h.quiesce();
      expect(page.manager.debug().live).not.toBe(firstLive);
      expect(page.timers.some((t) => !t.cancelled)).toBe(false); // 잔존 타이머 없음
    });

    it('R-D(restored 선착): 워치독을 취소한다', () => {
      const h = createHarness(2);
      const ed = {};
      const page = h.addPage(ed, 0);
      mountLiveGl(h, page);

      h.onContextLost(page);
      const watchdog = page.timers.find((t) => t.label === 'restore-watchdog');
      expect(watchdog?.cancelled).toBe(false);

      h.setAttachScript(page, ['gl']);
      h.onContextRestored(page);
      expect(watchdog?.cancelled).toBe(true);
      h.quiesce();
    });

    it('R-E: 강등-승격 게이트 — 처분 ack 전 대기자 미승격, ack 후에만 승격', () => {
      const h = createHarness(1);
      const a = {};
      const b = {};
      const pageA = h.addPage(a, 0);
      const pageB = h.addPage(b, 0);
      mountLiveGl(h, pageA);
      h.reconcile(pageB, inView);
      expect(h.pool.backendOf(b, 0)).toBe('cpu');

      h.setFocus(b);
      expect(h.pool.backendOf(a, 0)).toBe('cpu'); // 정책상 강등은 즉시 반영
      // 대기자는 실물 lease가 아직 살아있어(a의 release ack 전) 승격되지 않는다.
      expect(h.pool.backendOf(b, 0)).toBe('cpu');
      expect(pageA.manager.debug().live).toBeDefined(); // 실물 처분은 아직

      h.reconcile(pageA, outOfView); // a가 park되며 release 절차 시작
      h.flushDefer(pageA); // released ack — 이 시점에 정확히 1회, b로 슬롯이 즉시 이관(간접 승격 콜백)
      expect(h.pool.debugHoldCount()).toBe(1); // b의 신규 reserved lease
      expect(pageB.manager.debug().pending).toBeDefined();
      expect(pageB.attachRequests.at(-1)).toBe('gl'); // 콜백의 acquireHint를 그대로 소비(재협상 없음)

      h.flushDefer(pageB); // b의 attach ack
      h.firePresent(pageB); // b commit

      expect(h.pool.backendOf(b, 0)).toBe('gl');
      expect(pageB.manager.debug().live?.outcome).toBe('gl');
      h.quiesce();
    });

    it('R-F: 예산 소진 상태의 신규 페이지는 반환값 기반 cpu로 마운트되고 콜백은 발생하지 않는다', () => {
      const h = createHarness(1);
      const ed = {};
      const filler = h.addPage(ed, 0);
      mountLiveGl(h, filler);

      const fresh = h.addPage(ed, 1);
      h.setAttachScript(fresh, ['cpu']);
      h.reconcile(fresh, inView);
      expect(fresh.attachRequests).toEqual(['cpu']);
      expect(fresh.manager.debug().pending?.outcome).toBe('cpu');
      // 새 페이지 자신을 대상으로 한 승격 콜백은 없다(silent 반환값 경로).
      h.quiesce();
    });

    it('R-G: 연속 gl-dead는 즉시 cpu 마운트 1회와 dead lease release ack 1회로 수렴한다', () => {
      const h = createHarness(1);
      const ed = {};
      const page = h.addPage(ed, 0);
      h.setAttachScript(page, ['gl-dead']);
      h.reconcile(page, inView);

      expect(page.canvases).toHaveLength(2); // 죽은 gl 캔버스 + 즉시 폴백 cpu 캔버스
      expect(page.canvases[0].disposedGl).toBe(true);
      expect(page.canvases[0].removed).toBe(true);
      expect(page.canvases[1].removed).toBe(false);

      h.quiesce();
      h.firePresent(page);
      h.quiesce();
      expect(h.pool.debugHoldCount()).toBe(0);
      expect(page.manager.debug().live?.outcome).toBe('cpu');
    });

    it('R-H: 포커스 전환 스틸 — 구 보유자 선-강등 → 처분 ack → 신 포커스 승격, 매 단계 불변식 유지', () => {
      const h = createHarness(1);
      const a = {};
      const b = {};
      const pageA = h.addPage(a, 0);
      const pageB = h.addPage(b, 0);
      mountLiveGl(h, pageA);
      h.reconcile(pageB, inView);
      expect(h.pool.backendOf(b, 0)).toBe('cpu');

      h.setFocus(b);
      expect(h.pool.backendOf(a, 0)).toBe('cpu'); // 선-강등: 정책은 즉시, 실물은 대기
      expect(pageA.manager.debug().live).toBeDefined();

      h.clearFocus(a); // no-op: a는 애초에 focus를 가진 적 없다
      expect(h.pool.backendOf(a, 0)).toBe('cpu');

      h.reconcile(pageA, outOfView);
      h.flushDefer(pageA);
      expect(pageA.manager.debug().live).toBeUndefined();

      h.setAttachScript(pageB, ['gl']);
      h.reconcile(pageB, inView);
      h.flushDefer(pageB);
      h.firePresent(pageB);
      expect(h.pool.backendOf(b, 0)).toBe('gl');
      expect(pageB.manager.debug().live?.outcome).toBe('gl');
    });

    it.each([['acquire 이탈 후 재진입', 'reenter'] as const, ['이탈 없이 쿨다운 만료·wake 수신', 'wake'] as const])(
      'R-I(%s): 연속 타임아웃은 failedParked로 수렴하고 1회만 재시도한다',
      (_label, mode) => {
        const h = createHarness(1);
        const ed = {};
        const page = h.addPage(ed, 0);
        h.reconcile(page, inView); // 1차: gl pending

        h.fireTimerLabel(page, 'swap-timeout'); // 1차 타임아웃 → 강제 cpu 폴백
        h.fireTimerLabel(page, 'swap-timeout'); // 2차 타임아웃 → failedParked

        expect(page.manager.debug().live).toBeUndefined();
        expect(page.manager.debug().pending).toBeUndefined();
        expect(page.manager.debug().wantsLive).toBe(true);
        expect(page.timers.some((t) => !t.cancelled)).toBe(false);

        h.quiesce();

        let mountsBefore = page.canvases.length;
        if (mode === 'reenter') {
          // 짧은 backoff(1s)를 넘겨야 재진입이 즉시 gl을 받는다 — 그렇지 않으면 방금
          // 놓친 페이지의 thrash 방지 backoff에 걸려 이 테스트의 의도(재진입 자체가
          // 유효한 재시도 경로임을 확인)와 무관한 이유로 cpu를 받는다.
          h.advanceClock(1500);
          h.reconcile(page, outOfView); // acquire 반경 이탈
          h.setAttachScript(page, ['gl']);
          h.reconcile(page, inView); // 재진입
        } else {
          // "이탈 없이" 재시도하려면 풀 자신의 실패 쿨다운(3진 아웃)까지 도달해야 한다 —
          // swap timeout 경로는 풀 재배정을 거치지 않으므로(계약 5) 대상 엔트리의
          // backend는 여전히 stale gl로 남아있고, 3진 아웃의 강등 루프만이 이를
          // 정정해 만료 시 재승격 콜백을 내보낼 수 있다. 부족한 나머지 실패를 직접
          // noteGlFailure로 채워 3진 아웃 상태를 재현한다.
          h.pool.noteGlFailure(ed, 0, -1001);
          h.pool.noteGlFailure(ed, 0, -1002);
          expect(h.pool.backendOf(ed, 0)).toBe('cpu'); // 3진 아웃 강등이 stale 배정을 정정
          h.setAttachScript(page, ['gl']);
          h.quiesce();
          mountsBefore = page.canvases.length;
          h.advanceClock(40_000);
          h.firePoolTimers(); // 쿨다운 만료 wake — reconcile 재호출 없음
        }
        h.flushDefer(page);
        h.firePresent(page);
        expect(page.canvases.length).toBeGreaterThan(mountsBefore);
        expect(page.manager.debug().live).toBeDefined();
      },
    );

    it('R-J: pending 로스 × 스와프 타임아웃 동시 발생 시 failure count가 정확히 1 증가한다', () => {
      const runOnce = (lossFirst: boolean) => {
        const h = createHarness(2);
        const ed = {};
        const page = h.addPage(ed, 0);
        h.reconcile(page, inView); // gl pending, 미커밋

        if (lossFirst) {
          h.onContextLost(page);
          h.fireTimerLabel(page, 'swap-timeout');
        } else {
          h.fireTimerLabel(page, 'swap-timeout');
          h.onContextLost(page);
        }
        h.quiesce();

        // 두 사건(로스+타임아웃)이 정확히 1회만 계상됐다면, 이 시점의 실패 누적은 1이다.
        // 짧은 backoff(1s)를 넘긴 뒤 실착 실패를 정확히 1회 더 일으킨다 — 이중 계상이었다면
        // (누적 2에서 시작) 이 한 번으로 3진 아웃까지 조기 도달해 cpu에 갇힌다.
        h.advanceClock(1500);
        h.setAttachScript(page, ['cpu']);
        h.reconcile(page, outOfView);
        h.reconcile(page, inView);
        h.flushDefer(page); // ackAttached('cpu') 자체 강등 — 실패 누적 +1

        h.advanceClock(1500); // 이 실패의 짧은 backoff도 넘겨 3진 아웃 여부만으로 결과가 갈리게 한다
        h.reconcile(page, outOfView);
        h.reconcile(page, inView);
        return h.pool.backendOf(ed, 0);
      };

      // 정상: 누적 1(로스+타임아웃) + 1(실착) = 2 — 아직 3진 아웃 미도달, gl 재획득.
      // 이중 계상 버그라면: 누적 2 + 1 = 3 — 조기 쿨다운 진입, cpu에 갇힌다.
      expect(runOnce(true)).toBe('gl');
      expect(runOnce(false)).toBe('gl');
    });

    it('R-K: destroy 후 늦게 도착하는 attach ack는 무시되고, release ack는 orphan hold를 정확히 1회 소비한다', () => {
      const h = createHarness(1);
      const ed = {};
      const page = h.addPage(ed, 0);
      h.setAttachScript(page, ['gl']);
      h.reconcile(page, inView); // reserved, ack 미flush
      const token = page.canvases.at(-1)?.token;
      if (token === undefined) throw new Error('테스트 전제 위반');

      h.removeEditorAbrupt(ed); // pool 엔트리가 통째로 사라짐(관리자는 아직 모름)
      expect(h.pool.backendOf(ed, 0)).toBeUndefined();

      h.flushDefer(page); // 늦게 도착한 attach ack — 엔트리 부활 없이 무시(폰트 phase만 live로)
      expect(h.pool.backendOf(ed, 0)).toBeUndefined();
      expect(h.pool.debugHoldCount()).toBe(1); // orphan hold로 계상 유지

      h.destroyPage(page); // 뒤늦게 컴포넌트가 실제로 unmount
      h.flushDefer(page); // release ack — orphan hold를 1회 소비
      expect(h.pool.debugHoldCount()).toBe(0);

      h.pool.ackReleased(token); // 중복 ack — no-op
      expect(h.pool.debugHoldCount()).toBe(0);
    });

    // 스위트가 잡은 결함: gl 요청이 내부적으로 oversized cpu로 떨어지면(Rust FFI가 gl 실패 시
    // cpu로 조용히 폴백하고, 그 cpu 백킹이 예산을 초과하는 경우) page-surface-manager.ts의
    // cpu-oversized 조기 반환 분기가 예약 토큰을 ack/cancel 없이 버려 영구 누수시켰다.
    it('cpu-oversized: gl 요청이 oversized cpu로 떨어져도 예약 토큰이 새지 않는다', () => {
      const h = createHarness(1);
      const ed = {};
      const page = h.addPage(ed, 0);
      h.setAttachScript(page, ['cpu-oversized']);
      h.reconcile(page, inView);

      expect(h.pool.debugHoldCount()).toBe(0); // cancelReservation이 즉시 홀드를 소거했어야 한다
      expect(page.canvases).toHaveLength(1);
      expect(page.canvases[0].removed).toBe(true);
      expect(page.manager.debug().pending).toBeUndefined();
      expect(page.manager.debug().wantsLive).toBe(true); // 재시도는 radius 재진입에 맡긴다

      // 같은 페이지가 재시도해도 이전 토큰이 살아있어 예산을 잠식하지 않는다.
      h.setAttachScript(page, ['gl']);
      h.reconcile(page, outOfView);
      h.reconcile(page, inView);
      h.flushDefer(page);
      h.firePresent(page);
      expect(page.manager.debug().live?.outcome).toBe('gl');
      expect(h.pool.debugHoldCount()).toBe(1);
    });

    // 스위트가 잡은 결함: updatePageDemand의 silent 승격이 예산 가드보다 먼저 backend='gl'을
    // 찍어(원장 오염) 뒤따르는 acquireCanvasLease가 cpu를 돌려줘도 원장은 gl로 남았다. 슬롯이
    // 빌 때 #rebalance의 승격 루프가 backend==='gl'을 보고 건너뛰어 승격 콜백을 영영 안 내보내,
    // 정적 뷰(paint/typing이 reconcile을 안 부름)에서 포커스 페이지가 cpu에 영구히 갇혔다.
    it('R-L: A의 releasing 창 동안 고우선 B의 updateDemand는 원장을 오염시키지 않고, release ack 후 승격 콜백으로 realized-gl에 도달한다', () => {
      const h = createHarness(1);
      const a = {};
      const b = {};
      const pageA = h.addPage(a, 0);
      const pageB = h.addPage(b, 0);
      mountLiveGl(h, pageA); // A가 유일 슬롯을 gl live로 점유
      expect(h.pool.backendOf(a, 0)).toBe('gl');

      // B를 A보다 높은 우선순위(포커스)로 올린 뒤 그 상태에서 처음 진입시킨다 — A의 live lease가
      // 아직 예산을 점유한 창에서 B의 updateDemand(silent 경로)가 도는 상황.
      h.setFocus(b);
      h.setAttachScript(pageB, ['cpu']);
      h.reconcile(pageB, inView);

      // 오염 금지: 슬롯이 없으므로 B의 원장은 'gl'이 아니라 'cpu'로 남아야 한다.
      expect(h.pool.backendOf(b, 0)).toBe('cpu');
      h.flushDefer(pageB);
      h.firePresent(pageB);
      expect(pageB.manager.debug().live?.outcome).toBe('cpu'); // realized도 아직 cpu

      // A의 강등 폴백 commit → 구 gl live 처분(beginRelease) → release ack로 슬롯 해방.
      h.firePresent(pageA);
      h.flushDefer(pageA); // ackReleased → #rebalance → B로 간접 승격 콜백
      expect(pageB.manager.debug().pending?.outcome).toBe('gl');
      expect(pageB.attachRequests.at(-1)).toBe('gl'); // 콜백의 acquireHint를 그대로 소비

      h.flushDefer(pageB); // B의 gl ack
      h.firePresent(pageB); // B commit
      h.quiesce();

      // realized-gl 도달: 원장과 realized backend가 모두 gl로 합치.
      expect(h.pool.backendOf(b, 0)).toBe('gl');
      expect(pageB.manager.debug().live?.isGl).toBe(true);
      expect(pageB.manager.debug().live?.outcome).toBe('gl');
    });

    // 스위트가 잡은 결함(loss-recovery 변종): onContextRestored/watchdog가 stale 원장으로
    // mount(backendOf()==='gl')를 걸면 구 lease가 아직 예산을 물고 있어 acquire가 cpu로 떨어진다.
    // 이 cpu는 gl 실패가 아니라 예산 폴백이므로, 그 cpu가 커밋(finishSwap, 구 gl 표면 dispose 이후)
    // 될 때 manager가 noteBudgetFallback을 보내 원장을 un-poison한다. 신호가 없으면 원장이 'gl'로
    // 남아 슬롯이 다 비어도 #rebalance가 backend==='gl'을 보고 건너뛰어 정적 뷰에서 cpu에 갇힌다.
    it('R-M: loss-recovery 재마운트가 예산 폴백 cpu로 커밋되면 원인 태깅 un-poison으로 원장을 정정하고, 슬롯 해방 시 승격 콜백으로 realized-gl에 수렴한다', () => {
      const h = createHarness(1);
      const b = {};
      const pageB = h.addPage(b, 0);
      mountLiveGl(h, pageB); // B가 유일 슬롯을 gl live로 점유(원장 gl, lease 1)
      expect(h.pool.backendOf(b, 0)).toBe('gl');

      // 컨텍스트 복원 → mount(backendOf()==='gl'): 구 lease가 아직 예산을 물고 있어 acquire는 cpu.
      // 예산 폴백이므로 원장은 아직 'gl'로 정당하게 유지된다(구 gl live LB가 아직 살아있음).
      h.onContextRestored(pageB);
      expect(h.pool.backendOf(b, 0)).toBe('gl');
      expect(pageB.manager.debug().pending?.outcome).toBe('cpu'); // 폴백 pending은 cpu

      // 폴백 cpu commit → 구 gl LB 처분(beginRelease) → finishSwap이 noteBudgetFallback 발신 →
      // leaseless(LB는 releasing)이므로 원장을 'cpu'로 un-poison한다(오염 방지).
      h.firePresent(pageB);
      expect(h.pool.backendOf(b, 0)).toBe('cpu');
      expect(pageB.manager.debug().live?.outcome).toBe('cpu');

      h.setAttachScript(pageB, ['gl']);
      h.flushDefer(pageB); // ackReleased(LB) → 슬롯 해방 → rebalance가 B로 승격 콜백(재진입 reconcile 없이)
      expect(pageB.manager.debug().pending?.outcome).toBe('gl');
      expect(pageB.attachRequests.at(-1)).toBe('gl');
      expect(h.pool.backendOf(b, 0)).toBe('gl');

      h.flushDefer(pageB); // 승격 lease의 gl ack
      h.firePresent(pageB); // commit
      h.quiesce();

      // realized-gl 도달: 정적 뷰(reconcile 재호출 없음)에서도 승격 콜백만으로 수렴.
      expect(pageB.manager.debug().live?.isGl).toBe(true);
      expect(pageB.manager.debug().live?.outcome).toBe('gl');
      expect(h.pool.debugHoldCount()).toBe(1);
    });

    // 계약 5항 대조군: 같은 "구 gl 표면 → cpu commit → 슬롯 해방" 시퀀스라도, cpu가 gl '실패'
    // (swap timeout) 강제 폴백이면 noteBudgetFallback을 보내지 '않아' 원장이 stale 'gl'로 남는다.
    // 따라서 슬롯이 비어도 승격 콜백이 나가지 않는다(풀 재진입 금지) — 재시도는 reconcile/쿨다운
    // 웨이크에만 맡긴다. 이 대조가 태깅(원인 구분)이 실제로 동작함을 보증한다.
    it('R-M(대조군): timeout 강제 cpu 폴백은 noteBudgetFallback을 보내지 않아 슬롯이 비어도 승격 콜백을 받지 않는다(계약 5)', () => {
      const h = createHarness(1);
      const c = {};
      const pageC = h.addPage(c, 0);
      h.reconcile(pageC, inView); // gl pending(예약), 미커밋
      expect(h.pool.debugHoldCount()).toBe(1);

      h.fireTimerLabel(pageC, 'swap-timeout'); // 1차 타임아웃 → 예약 취소 + 강제 cpu 폴백(budgetFallback=false)
      h.firePresent(pageC); // 강제 cpu commit — finishSwap이 noteBudgetFallback을 보내지 않는다

      // 슬롯은 비었지만(holdCount 0) 원장은 stale 'gl'로 남아 승격 콜백이 없다.
      expect(h.pool.debugHoldCount()).toBe(0);
      expect(h.pool.backendOf(c, 0)).toBe('gl'); // un-poison되지 않음(계약 5)
      expect(pageC.manager.debug().live?.outcome).toBe('cpu');

      h.flushDefer(pageC); // 지연된 noteGlFailure 소진 — 그래도 승격 콜백은 없다
      h.quiesce();
      expect(pageC.manager.debug().pending).toBeUndefined(); // gl 재마운트(승격 콜백) 없음
      expect(pageC.manager.debug().live?.outcome).toBe('cpu');
      expect(h.pool.debugHoldCount()).toBe(0);
    });
  });

  describe('Step 2e: 우선순위 정책 정확성(참조 모델)', () => {
    it.each([
      [
        '4-tier 전 조합: 포커스 가시 > 포커스 overscan > 비포커스 가시 > 비포커스 overscan',
        [
          { id: 'a', editorKey: 'A', zone: 'overscan' as PageZone, isHolder: false, lastPresent: 0 },
          { id: 'b', editorKey: 'B', zone: 'visible' as PageZone, isHolder: false, lastPresent: 0 },
          { id: 'c', editorKey: 'A', zone: 'visible' as PageZone, isHolder: false, lastPresent: 0 },
          { id: 'd', editorKey: 'B', zone: 'overscan' as PageZone, isHolder: false, lastPresent: 0 },
        ],
        'A',
        2,
        ['c', 'a'],
      ],
      [
        'holder-vs-waiter: 동순위에서 보유자가 대기자를 이긴다',
        [
          { id: 'holder', editorKey: 'A', zone: 'visible' as PageZone, isHolder: true, lastPresent: 0 },
          { id: 'waiter', editorKey: 'A', zone: 'visible' as PageZone, isHolder: false, lastPresent: 5 },
        ],
        null,
        1,
        ['holder'],
      ],
      [
        'holder LRU: 보유자끼리는 최근 present가 늦은 쪽이 이긴다',
        [
          { id: 'stale-holder', editorKey: 'A', zone: 'visible' as PageZone, isHolder: true, lastPresent: 1 },
          { id: 'fresh-holder', editorKey: 'A', zone: 'visible' as PageZone, isHolder: true, lastPresent: 9 },
        ],
        null,
        1,
        ['fresh-holder'],
      ],
      [
        'waiter LRU: 대기자끼리도 최근 present가 늦은 쪽이 이긴다',
        [
          { id: 'stale-waiter', editorKey: 'A', zone: 'visible' as PageZone, isHolder: false, lastPresent: 1 },
          { id: 'fresh-waiter', editorKey: 'A', zone: 'visible' as PageZone, isHolder: false, lastPresent: 9 },
        ],
        null,
        1,
        ['fresh-waiter'],
      ],
      [
        '무압박 focus clear: 예산이 넉넉하면 focus 유무가 승자 집합을 바꾸지 않는다',
        [
          { id: 'a', editorKey: 'A', zone: 'visible' as PageZone, isHolder: true, lastPresent: 0 },
          { id: 'b', editorKey: 'A', zone: 'visible' as PageZone, isHolder: true, lastPresent: 0 },
        ],
        null,
        2,
        ['a', 'b'],
      ],
    ] as const)('%s', (_label, entries, focused, budget, expectedIds) => {
      const winners = expectedWinners(entries as unknown as PolicyEntry[], focused as unknown as object | null, budget);
      const ids = new Set([...winners].map((e) => (e as unknown as { id: string }).id));
      expect(ids).toEqual(new Set(expectedIds));
    });

    it('랜덤 연산열의 매 quiescence 시점마다 desiredBackend 집합이 모델과 일치한다', () => {
      // gl 실패를 유발하지 않는 순수 정책 시나리오로 한정한다 — 모델은 실패/backoff 이력을
      // 모르므로, 실패가 섞이면 정당한 backoff 배제가 "모델과 불일치"로 오판된다.
      const budget = 2;
      const h = createHarness(budget);
      const editors = [{}, {}];
      const pages = editors.flatMap((ed, ei) => [h.addPage(ed, 0), h.addPage(ed, 1), h.addPage(ed, 2)].map((p) => ({ p, ei })));

      // present는 순수 정책 사건이 아니라("swap 확정"이라는 별도의 실세계 이벤트) 무작위
      // 선택만으로는 좀처럼 모든 페이지가 동시에 정착하지 않는다 — 주기적으로 모든 pending을
      // 강제 정착시켜(present) 의미 있는 quiescence 체크포인트를 확보한다.
      const settleAll = () => {
        for (let i = 0; i < 20; i++) {
          h.flushAllDefer();
          let progressed = false;
          for (const { p } of pages) {
            if (p.presented.length === 0) {
              continue;
            }

            h.firePresent(p);
            progressed = true;
          }
          if (!progressed) return;
        }
      };

      const rng = mulberry32(0xc0_ff_ee);
      let focused: object | null = null;
      let strictCheckpoints = 0;
      let nonEmptyStrictCheckpoints = 0;

      for (let step = 0; step < 60; step++) {
        const pick = pages[Math.floor(rng() * pages.length)];
        const roll = rng();
        if (roll < 0.45) {
          const visRoll = rng();
          const visibility = visRoll < 0.34 ? inView : visRoll < 0.67 ? overscan : outOfView;
          h.reconcile(pick.p, visibility);
        } else if (roll < 0.7) {
          h.flushDefer(pick.p);
        } else if (roll < 0.85) {
          h.firePresent(pick.p);
        } else if (roll < 0.92) {
          focused = editors[Math.floor(rng() * editors.length)];
          h.setFocus(focused);
        } else {
          if (focused) h.clearFocus(focused);
          focused = null;
        }

        // 약한 방향은 항상: 실제 gl 보유 집합은 모델의 당첨자 집합의 부분집합이어야 한다
        // (release 게이트로 지연 중인 승격만 실제-모델 괴리를 만들 수 있고, 그 괴리는 항상
        // "모델 당첨인데 아직 cpu" 방향이지 그 반대가 아니다).
        const actual = currentDesiredGlSet(h);
        const model = modelWinnersFor(h, focused, budget);
        for (const key of actual) expect(model.has(key)).toBe(true);

        if (step % 5 === 4) {
          settleAll();
          if (h.isFullyQuiescent()) {
            // 강한 방향: 모든 release/present가 정착된 완전한 quiescence에서는 정확히 일치해야 한다.
            // 원장뿐 아니라 realized backend까지 원장과 합치하는지 확인해 silent 오염을 잡는다.
            assertRealizedMatchesLedger(h);
            const settledActual = currentDesiredGlSet(h);
            const settledModel = modelWinnersFor(h, focused, budget);
            const sortStrings = (values: string[]) => values.toSorted((a, b) => a.localeCompare(b));
            if (JSON.stringify(sortStrings([...settledActual])) !== JSON.stringify(sortStrings([...settledModel]))) {
              throw new Error(
                `모델 불일치(step=${step}): actual=${JSON.stringify([...settledActual])} model=${JSON.stringify([...settledModel])}\n` +
                  `opLog:\n${h.opLog.join('\n')}`,
              );
            }
            strictCheckpoints += 1;
            if (settledActual.size > 0) nonEmptyStrictCheckpoints += 1;
          }
        }
      }

      expect(strictCheckpoints).toBeGreaterThan(0); // 시나리오가 실제로 quiescence에 도달했는지 확인
      expect(nonEmptyStrictCheckpoints).toBeGreaterThan(0); // 공집합 대 공집합의 공허한 일치가 아님을 확인
    });
  });

  describe('Step 3: property 테스트(랜덤 연산열)', () => {
    // reconcile in/out/overscan, attach 결과 스크립트 push, defer flush(전체/개별), 타이머(페이지/풀),
    // 시계 전진, 로스/복원, focus 전환, removeEditor/destroy(늦은 ack 인터리빙 포함)를 모두 아우르는
    // 연산 어휘 — 매 호출이 하니스 wrapper를 거치므로 자동으로 I1~I6(안전성)이 assert된다. I7
    // (liveness)은 안전성과 별개로 주기적 settle 체크포인트(아래 settleForLiveness+assertLiveness,
    // runPropertySequence에서 50스텝마다 + 말미에 호출)가 담당한다 — "안전하지만 영구 정지"는
    // I1~I6만으로는 절대 걸리지 않기 때문이다. 위반 시 seed와 마지막 연산열을 출력해 재현 가능하게 한다.

    // I7 체크포인트 전용 settle: 존재하는 pending을 전부 present로 커밋한다(present는 실세계에서
    // 매 프레임 자동 발생하는 렌더 커밋에 대응하므로 가짜 클록을 앞당기지 않고 즉시 발화해도
    // 안전하다) + defer와 "이미 due한" 타이머만 가짜 클록을 조작하지 않고 소진한다(quiesce).
    // 절대로 아직 만료되지 않은 swap-timeout을 강제로 앞당겨 발화시키지 않는다 — 그러면 실제로는
    // present가 먼저 성공했을 조합을 인위적인 2차 타임아웃으로 오염시켜 거짓 failedParked를
    // 만들어낼 수 있다(present 우선 발화로 이 위험을 원천 차단).
    function settleForLiveness(h: Harness): void {
      for (let round = 0; round < 100; round++) {
        h.quiesce();
        let firedPresent = false;
        for (const page of h.knownPages()) {
          if (page.presented.length === 0) continue;
          h.firePresent(page);
          firedPresent = true;
        }
        if (!firedPresent) return;
      }
      throw new Error('I7 체크포인트 settle이 100회 반복 내에 수렴하지 않음(thrash 의심)');
    }

    // settle 후: inAcquire로 마지막에 reconcile된 페이지는 quiescence 시점에 정확히 하나의 live
    // surface를 가져야 한다 — 예외는 failedParked(계약 5의 안정 상태)뿐이다. 관리자 내부를 뜯지
    // 않고 관측 가능한 신호(live/pending 모두 없음 + wantsLive 유지)만으로 판별한다: settle이
    // 이미 즉시 가용한 모든 재시도 경로(현재 pending의 present 커밋, 이미 due한 타이머·풀 웨이크
    // 캐스케이드)를 소진했으므로, 그러고도 남아있는 "live/pending 없음" 상태는 곧
    // !retryEligible(남은 경로는 재진입 reconcile 또는 아직 만료 안 된 쿨다운뿐)과 동치다.
    function assertLiveness(h: Harness): void {
      for (const page of h.activePages()) {
        if (!page.state.lastVisibility?.inAcquire) continue;
        const debug = page.manager.debug();
        const failedParked = debug.wantsLive && debug.live === undefined && debug.pending === undefined;
        if (failedParked) continue;
        expect(debug.live).toBeDefined();
        expect(debug.pending).toBeUndefined();
        // realized backend↔원장 정합: settle 시점의 live가 gl이면 원장도 반드시 gl이어야 한다
        // (강등은 settle에서 cpu 커밋으로 완료되므로 gl live가 원장-cpu와 함께 남지 않는다).
        // 역방향(원장 gl·realized cpu)은 예산 압박 하 gl 복구 재마운트가 cpu로 폴백한 정당한
        // 퇴화라 여기서 강제하지 않는다 — silent 오염 회귀는 R-L과 Step 2e realized==ledger가 잡는다.
        if (debug.live?.isGl) expect(h.pool.backendOf(page.editorKey, page.page)).toBe('gl');
      }
    }

    function runPropertySequence(seed: number, steps: number, budget: number): void {
      const h = createHarness(budget);
      const editors = [{}, {}];
      const pagesList: PageHandle[][] = editors.map((ed) => [
        h.addPage(ed, 0, seed % 2 === 0),
        h.addPage(ed, 1, seed % 3 === 0),
        h.addPage(ed, 2),
      ]);
      const rng = mulberry32(seed);
      let focused: object | null = null;

      try {
        for (let step = 0; step < steps; step++) {
          const ei = Math.floor(rng() * editors.length);
          const pi = Math.floor(rng() * pagesList[ei].length);
          const page = pagesList[ei][pi];
          const roll = rng();

          if (roll < 0.26) {
            const visRoll = rng();
            const visibility = visRoll < 0.34 ? inView : visRoll < 0.67 ? overscan : outOfView;
            h.reconcile(page, visibility);
          } else if (roll < 0.38) {
            h.flushDefer(page);
          } else if (roll < 0.44) {
            h.flushAllDefer();
          } else if (roll < 0.52) {
            h.firePresent(page);
          } else if (roll < 0.6) {
            h.fireAllTimers(page);
          } else if (roll < 0.65) {
            h.firePoolTimers();
          } else if (roll < 0.71) {
            h.advanceClock(Math.floor(rng() * 1200));
          } else if (roll < 0.76) {
            h.onContextLost(page);
          } else if (roll < 0.8) {
            h.onContextRestored(page);
          } else if (roll < 0.86) {
            focused = editors[Math.floor(rng() * editors.length)];
            h.setFocus(focused);
          } else if (roll < 0.9) {
            if (focused) h.clearFocus(focused);
            focused = null;
          } else if (roll < 0.95) {
            const outcomeRoll = rng();
            const outcome: ScriptedOutcome =
              outcomeRoll < 0.55 ? 'gl' : outcomeRoll < 0.75 ? 'cpu' : outcomeRoll < 0.9 ? 'gl-dead' : 'cpu-oversized';
            h.setAttachScript(page, [outcome]);
          } else if (roll < 0.98) {
            // destroy(removeEditor의 페이지 단위 대응) — 즉시 재생성해 좌표를 안정적으로 유지한다.
            pagesList[ei][pi] = h.recyclePage(page, rng() < 0.5);
          } else {
            // removeEditor(늦은 ack 인터리빙의 핵심) — 에디터 전체가 통째로 사라진 뒤 재구성된다.
            const ed = editors[ei];
            h.removeEditorAbrupt(ed);
            if (focused === ed) focused = null;
            pagesList[ei] = [h.addPage(ed, 0, rng() < 0.5), h.addPage(ed, 1, rng() < 0.5), h.addPage(ed, 2, rng() < 0.5)];
          }

          // I7 liveness 체크포인트 — 50스텝마다. I1~I6는 매 연산 후 자동 assert되지만 "안전하지만
          // 영구 정지"는 그것만으로 못 잡는다.
          if (step % 50 === 49) {
            settleForLiveness(h);
            assertLiveness(h);
          }
        }

        // 시퀀스 말미에도 한 번 더 — 마지막 체크포인트 이후 스텝들이 새 정지를 만들었을 수 있다.
        settleForLiveness(h);
        assertLiveness(h);
      } catch (err) {
        const message = err instanceof Error ? (err.stack ?? err.message) : String(err);
        throw new Error(
          `property 테스트 실패(seed=${seed}, steps=${steps}, budget=${budget}):\n${message}\n\n` +
            `opLog(마지막 80개):\n${h.opLog.slice(-80).join('\n')}`,
          { cause: err },
        );
      }
    }

    // 예산 3곳(경쟁 있음/극한 경쟁/무압박에 가까움)에 걸쳐 각각 여러 시드 × 400스텝을 고정
    // 배치한다. 실패 시 위 catch가 seed·steps·budget과 opLog 꼬리를 출력해 재현 가능하다.
    it.each(Array.from({ length: 8 }, (_, i) => i + 1))('시드 %i: 400스텝 동안 I1~I7이 유지된다(budget 3)', (seed) => {
      runPropertySequence(seed * 0x9e_37_79_b1, 400, 3);
    });
    it.each(Array.from({ length: 5 }, (_, i) => i + 100))('시드 %i: budget 1(극한 경쟁)에서도 400스텝 동안 I1~I7이 유지된다', (seed) => {
      runPropertySequence(seed * 0x9e_37_79_b1, 400, 1);
    });
    it.each(Array.from({ length: 5 }, (_, i) => i + 200))(
      '시드 %i: budget 6(무압박에 가까움)에서도 400스텝 동안 I1~I7이 유지된다',
      (seed) => {
        runPropertySequence(seed * 0x9e_37_79_b1, 400, 6);
      },
    );
  });
});
