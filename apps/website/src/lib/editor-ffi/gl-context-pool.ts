export type SurfaceBackend = 'gl' | 'cpu';
export type PageZone = 'visible' | 'overscan';

// 개별 GL 캔버스를 가리키는 불투명 식별자. 토큰 자체가 세대 역할을 겸한다 — 같은 페이지라도
// GL→GL 재마운트 중에는 구/신 토큰이 서로 다른 값으로 공존하므로 세대 필드를 별도로 둘 필요가 없다.
export type LeaseToken = number;

export type AttachOutcome = 'gl' | 'cpu' | 'gl-dead';
export type CancelReason = string;

export type BackendChangeHandler = (editorKey: object, page: number, backend: SurfaceBackend, acquireHint?: LeaseToken) => void;

export type Schedule = (fn: () => void, ms: number) => () => void;

const defaultSchedule: Schedule = (fn, ms) => {
  const id = setTimeout(fn, ms);
  return () => clearTimeout(id);
};

export type LeasePhase = 'reserved' | 'live' | 'releasing';

type LeaseRecord = {
  token: LeaseToken;
  editorKey: object;
  page: number;
  phase: LeasePhase;
};

// 테스트 전용 introspection: 토큰=세대이므로 leaseId와 generation은 같은 값을 담는다.
export type LeaseSnapshot = { leaseId: LeaseToken; generation: number; phase: LeasePhase; editorKey: object; page: number };

type PageEntry = {
  editorKey: object;
  page: number;
  zone: PageZone;
  backend: SurfaceBackend;
  currentToken: LeaseToken | null;
  lastPresent: number;
};

type FailureRecord = {
  consecutive: number;
  blockedUntil: number;
  shortBackoffUntil: number;
};

type PendingChange = [editorKey: object, page: number, backend: SurfaceBackend, acquireHint: LeaseToken | undefined];

const GL_FAILURE_THRESHOLD = 3;
const GL_FAILURE_COOLDOWN_MS = 30_000;
const GL_FAILURE_SHORT_BACKOFF_MS = 1000;

// lease 계약: 예산은 reserved+live+releasing(orphan 포함)의 합으로 계상하고, 승격은 ackReleased
// 이후에만 일어난다. updatePageDemand는 정책(어느 페이지가 gl 배정을 받을지)만 결정하며, 실제
// 슬롯 예약은 acquireCanvasLease가 호출 시점의 원장을 다시 확인해 담당한다 — 그래서 정책상
// 'gl'이어도 releasing lease가 아직 안 빠졌으면 acquireCanvasLease는 일시적으로 'cpu'를 돌려준다.
export class GlContextPool {
  #budget: number;
  #onBackendChange: BackendChangeHandler;
  #now: () => number;
  #schedule: Schedule;
  #disabled: boolean;

  #entries = new Map<object, Map<number, PageEntry>>();
  #leases = new Map<LeaseToken, LeaseRecord>();
  #failures = new Map<object, Map<number, FailureRecord>>();
  #countedFailureTokens = new Set<LeaseToken>();
  #focused: object | null = null;
  #presentClock = 0;
  #tokenCounter = 0;
  #pending: PendingChange[] = [];
  #flushing = false;
  #cancelWake: (() => void) | undefined;

  constructor(
    budget: number,
    onBackendChange: BackendChangeHandler,
    now: () => number = () => Date.now(),
    schedule: Schedule = defaultSchedule,
    disabled = false,
  ) {
    this.#budget = budget;
    this.#onBackendChange = onBackendChange;
    this.#now = now;
    this.#schedule = schedule;
    this.#disabled = disabled;
  }

  #reserve(entry: PageEntry): LeaseToken {
    const token = ++this.#tokenCounter;
    this.#leases.set(token, { token, editorKey: entry.editorKey, page: entry.page, phase: 'reserved' });
    entry.currentToken = token;
    return token;
  }

  #retire(lease: LeaseRecord): void {
    this.#leases.delete(lease.token);
    this.#countedFailureTokens.delete(lease.token);
  }

  #clearCurrentToken(lease: LeaseRecord, token: LeaseToken): void {
    const entry = this.#entries.get(lease.editorKey)?.get(lease.page);
    if (entry?.currentToken === token) entry.currentToken = null;
  }

  #recordFailureOnce(editorKey: object, page: number, incident: LeaseToken): boolean {
    if (this.#countedFailureTokens.has(incident)) return false;
    this.#countedFailureTokens.add(incident);
    this.#recordFailure(editorKey, page);
    return true;
  }

  #recordFailure(editorKey: object, page: number): void {
    let pages = this.#failures.get(editorKey);
    if (!pages) {
      pages = new Map();
      this.#failures.set(editorKey, pages);
    }
    const record = pages.get(page) ?? { consecutive: 0, blockedUntil: 0, shortBackoffUntil: 0 };
    record.consecutive += 1;
    record.shortBackoffUntil = this.#now() + GL_FAILURE_SHORT_BACKOFF_MS;
    if (record.consecutive >= GL_FAILURE_THRESHOLD) {
      record.blockedUntil = this.#now() + GL_FAILURE_COOLDOWN_MS;
    }
    pages.set(page, record);
  }

  #blocked(entry: PageEntry): boolean {
    const record = this.#failures.get(entry.editorKey)?.get(entry.page);
    if (!record) return false;
    const now = this.#now();
    if (record.consecutive >= GL_FAILURE_THRESHOLD && now < record.blockedUntil) return true;
    // 현재 보유자는 짧은 backoff에서 면제 — 이 backoff는 방금 놓친 페이지가 곧바로 재획득해
    // 실패를 무한 반복하는 thrash를 막기 위한 것이지, 이미 붙어 있는 페이지를 쫓아내는 용도가 아니다.
    return entry.backend !== 'gl' && now < record.shortBackoffUntil;
  }

  #tier(entry: PageEntry): number {
    const focused = entry.editorKey === this.#focused;
    if (focused) return entry.zone === 'visible' ? 0 : 1;
    return entry.zone === 'visible' ? 2 : 3;
  }

  #allEntries(): PageEntry[] {
    return [...this.#entries.values()].flatMap((pages) => [...pages.values()]);
  }

  #rebalance(options?: { silent?: PageEntry }): void {
    const all = this.#allEntries();
    const eligible = all.filter((entry) => !this.#blocked(entry));
    eligible.sort(
      (a, b) => this.#tier(a) - this.#tier(b) || Number(b.backend === 'gl') - Number(a.backend === 'gl') || b.lastPresent - a.lastPresent,
    );
    const desired = this.#disabled ? new Set<PageEntry>() : new Set(eligible.slice(0, this.#budget));

    let usage = this.#leases.size;

    for (const entry of eligible) {
      if (!desired.has(entry) || entry.backend === 'gl') continue;
      if (usage >= this.#budget) continue;
      if (entry === options?.silent) {
        // 직접 호출자 자신의 승격은 반환값으로 전달되므로 콜백을 큐잉하지 않고, 실제 예약도
        // 뒤따르는 acquireCanvasLease 호출에 맡긴다(중복 예약 방지). 다만 슬롯 계상(usage)은
        // 지금 반영해야 같은 rebalance의 다른 대기자가 이 슬롯을 가로채지 않고, 예산이 없으면
        // 위 가드에 걸려 'cpu'로 남아 이후 슬롯이 빌 때 승격 콜백을 받는다(원장 오염 방지).
        usage += 1;
        entry.backend = 'gl';
        continue;
      }
      const token = this.#reserve(entry);
      usage += 1;
      entry.backend = 'gl';
      this.#pending.push([entry.editorKey, entry.page, 'gl', token]);
    }

    for (const entry of all) {
      if (desired.has(entry) || entry.backend !== 'gl') continue;
      entry.backend = 'cpu';
      if (entry === options?.silent) continue;
      this.#pending.push([entry.editorKey, entry.page, 'cpu', undefined]);
    }

    this.#scheduleCooldownWake(all);
    this.#flush();
  }

  #scheduleCooldownWake(all: PageEntry[]): void {
    this.#cancelWake?.();
    this.#cancelWake = undefined;
    const now = this.#now();
    let earliest = Infinity;
    for (const entry of all) {
      const record = this.#failures.get(entry.editorKey)?.get(entry.page);
      if (!record) continue;
      if (record.consecutive >= GL_FAILURE_THRESHOLD && record.blockedUntil > now) {
        earliest = Math.min(earliest, record.blockedUntil);
      } else if (record.shortBackoffUntil > now) {
        earliest = Math.min(earliest, record.shortBackoffUntil);
      }
    }
    if (Number.isFinite(earliest)) {
      this.#cancelWake = this.#schedule(() => this.#rebalance(), Math.max(0, earliest - now));
    }
  }

  #flush(): void {
    if (this.#flushing) return;
    this.#flushing = true;
    try {
      while (this.#pending.length > 0) {
        const next = this.#pending.shift();
        if (!next) break;
        const [editorKey, page, backend, acquireHint] = next;
        if (this.#entries.get(editorKey)?.get(page)?.backend !== backend) continue;
        this.#onBackendChange(editorKey, page, backend, acquireHint);
      }
    } finally {
      this.#flushing = false;
    }
  }

  updatePageDemand(editorKey: object, page: number, zone: PageZone): SurfaceBackend {
    let pages = this.#entries.get(editorKey);
    if (!pages) {
      pages = new Map();
      this.#entries.set(editorKey, pages);
    }
    let entry = pages.get(page);
    if (entry) {
      entry.zone = zone;
    } else {
      entry = { editorKey, page, zone, backend: 'cpu', currentToken: null, lastPresent: 0 };
      pages.set(page, entry);
    }
    this.#rebalance({ silent: entry });
    return entry.backend;
  }

  // 캔버스 1개를 생성하기 직전에 호출한다 — 매 호출이 독립된 새 lease(신규 토큰)에 대응한다.
  // GL→GL 재마운트에서도 구 lease는 손대지 않고 새 예약만 시도하므로 둘이 세대를 공유하지 않는다.
  acquireCanvasLease(
    editorKey: object,
    page: number,
    requested: SurfaceBackend,
  ): { backend: 'cpu' } | { backend: 'gl'; token: LeaseToken } {
    if (requested !== 'gl') return { backend: 'cpu' };
    const entry = this.#entries.get(editorKey)?.get(page);
    if (!entry || entry.backend !== 'gl') return { backend: 'cpu' };
    if (this.#leases.size >= this.#budget) return { backend: 'cpu' };

    const token = this.#reserve(entry);
    return { backend: 'gl', token };
  }

  // 모든 예약 기반 attach 결과를 ack한다(정상 일치 포함). stale/취소된 토큰은 조용히 무시.
  ackAttached(token: LeaseToken, actual: AttachOutcome): void {
    const lease = this.#leases.get(token);
    if (!lease) return;

    if (actual === 'gl') {
      lease.phase = 'live';
      return;
    }

    if (actual === 'cpu') {
      // 예약된 gl 슬롯에 cpu가 실제로 붙었다 — 컨텍스트가 없으므로 released 왕복 없이 즉시 반납.
      this.#retire(lease);
      this.#clearCurrentToken(lease, token);
      const entry = this.#entries.get(lease.editorKey)?.get(lease.page);
      if (entry) entry.backend = 'cpu';
      this.#recordFailureOnce(lease.editorKey, lease.page, token);
      this.#rebalance();
      return;
    }

    // 'gl-dead': 캔버스는 생겼으나 즉시 죽었다 — 반납은 released ack까지 미루되(원장 계상 유지),
    // 실패는 지금 기록한다.
    lease.phase = 'releasing';
    this.#recordFailureOnce(lease.editorKey, lease.page, token);
    this.#rebalance();
  }

  // ack 전 취소 — 예약을 원자적으로 소거한다. 실패로 계상하지 않는다(정책적 포기이지 GL 결함이 아님).
  cancelReservation(token: LeaseToken, reason: CancelReason): void {
    void reason;
    const lease = this.#leases.get(token);
    if (!lease || lease.phase !== 'reserved') return;
    this.#retire(lease);
    this.#clearCurrentToken(lease, token);
    this.#rebalance();
  }

  // 처분 시작 — releasing으로 전이할 뿐 원장 계상은 그대로 유지된다(승격은 ackReleased까지 대기).
  beginRelease(token: LeaseToken): void {
    const lease = this.#leases.get(token);
    if (!lease) return;
    lease.phase = 'releasing';
  }

  // 처분 완료 ack — 이 시점에만 슬롯이 실제로 비고 대기자가 승격된다. 중복/stale 토큰은 Map에서
  // 이미 사라졌으므로 자연히 무시된다(removeEditor 뒤 첫 ack만 유효하게 소비되는 이유).
  ackReleased(token: LeaseToken): void {
    const lease = this.#leases.get(token);
    if (!lease) return;
    this.#retire(lease);
    this.#clearCurrentToken(lease, token);
    this.#rebalance();
  }

  backendOf(editorKey: object, page: number): SurfaceBackend | undefined {
    return this.#entries.get(editorKey)?.get(page)?.backend;
  }

  // token 없이 호출하면 recency만 갱신(에디터의 매 commit 훅). token이 현재 페이지의 live lease와
  // 일치하면 실패 이력까지 리셋한다(스와프가 committed present로 확정된 시점).
  notePresent(editorKey: object, page: number, token?: LeaseToken): void {
    const entry = this.#entries.get(editorKey)?.get(page);
    if (!entry) return;
    entry.lastPresent = ++this.#presentClock;
    if (token === undefined || entry.currentToken !== token) return;
    if (this.#leases.get(token)?.phase === 'live') {
      this.#failures.get(editorKey)?.delete(page);
    }
  }

  // incident(=lease 토큰 상당)당 최대 1회만 계상한다 — 같은 pending의 로스와 타임아웃 워치독이
  // 이중 계상되어 3회 강등이 조기 발동하는 것을 막는다.
  noteGlFailure(editorKey: object, page: number, incident: LeaseToken): void {
    if (!this.#recordFailureOnce(editorKey, page, incident)) return;
    this.#rebalance();
  }

  // manager가 gl을 요청했지만 예산 부족으로 cpu가 실제 커밋됐다는 신호(구 gl 표면 dispose 이후).
  // 이 시점에 entry는 leaseless인데 원장은 stale 'gl'로 남아, 슬롯이 비어도 #rebalance 승격 루프가
  // backend==='gl'을 보고 건너뛴다 — 원장을 'cpu'로 un-poison하고 1회 재조정해 대기자로 되돌린다
  // (슬롯이 이미 비었으면 여기서 즉시 승격 콜백). 실패(timeout/gl-dead) 경로는 이 신호를 보내지
  // 않으므로(계약 5) 강제 cpu 폴백은 여전히 stale 'gl'로 남아 3진 강등/재진입으로만 정정된다.
  noteBudgetFallback(editorKey: object, page: number): void {
    const entry = this.#entries.get(editorKey)?.get(page);
    if (!entry || entry.backend !== 'gl') return;
    // 이 좌표에 아직 live/reserved lease가 있으면 원장 'gl'은 정당하다 — un-poison 금지. 이는 좌표
    // 재사용(removeEditor 후 재구성) 뒤 옛 핸들의 뒤늦은 budget-fallback 커밋이 실제 gl을 보유한 새
    // 엔트리를 오강등하는 것을 막는다. 처분 중(releasing)만 남았거나 leaseless일 때만 강등한다.
    for (const lease of this.#leases.values()) {
      if (lease.editorKey === editorKey && lease.page === page && lease.phase !== 'releasing') return;
    }
    entry.backend = 'cpu';
    this.#rebalance();
  }

  setFocus(editorKey: object): void {
    if (this.#focused === editorKey) return;
    this.#focused = editorKey;
    this.#rebalance();
  }

  clearFocus(editorKey: object): void {
    if (this.#focused !== editorKey) return;
    this.#focused = null;
    this.#rebalance();
  }

  // 파킹 — 엔트리는 지우되 실패 이력은 보존한다. 보유 중이던 lease는 #leases에 그대로 남아
  // (엔트리와 독립적으로 추적되므로) ack 전이든 후든 새는 홀드가 생기지 않는다.
  leave(editorKey: object, page: number): void {
    const pages = this.#entries.get(editorKey);
    if (!pages?.has(page)) return;
    pages.delete(page);
    if (pages.size === 0) this.#entries.delete(editorKey);
    this.#rebalance();
  }

  // 영구 제거 — 실패 이력까지 정리한다.
  forget(editorKey: object, page: number): void {
    this.leave(editorKey, page);
    const failures = this.#failures.get(editorKey);
    failures?.delete(page);
    if (failures && failures.size === 0) this.#failures.delete(editorKey);
  }

  removeEditor(editorKey: object): void {
    this.#entries.delete(editorKey);
    this.#failures.delete(editorKey);
    if (this.#focused === editorKey) this.#focused = null;
    this.#rebalance();
  }

  debugHoldCount(): number {
    return this.#leases.size;
  }

  // 테스트 전용: 현재 원장의 lease 레코드 스냅샷(I6 단계별 보존 검증용).
  debugLeaseSnapshot(): LeaseSnapshot[] {
    return [...this.#leases.values()].map((lease) => ({
      leaseId: lease.token,
      generation: lease.token,
      phase: lease.phase,
      editorKey: lease.editorKey,
      page: lease.page,
    }));
  }
}

let handler: BackendChangeHandler | undefined;
export function setBackendChangeHandler(next: BackendChangeHandler): void {
  handler = next;
}
const forceCpu = typeof localStorage !== 'undefined' && localStorage.getItem('typie:page-surface') === 'cpu';
// 스펙 §3.4 예산 장부: 브라우저 한도(≥8) − 프로브/GC 마진 → 8 (구 공유 presenter 제거 완료).
export const GL_POOL_BUDGET = 8;
export const glContextPool = new GlContextPool(
  GL_POOL_BUDGET,
  (editorKey, page, backend, acquireHint) => handler?.(editorKey, page, backend, acquireHint),
  undefined,
  undefined,
  forceCpu,
);
