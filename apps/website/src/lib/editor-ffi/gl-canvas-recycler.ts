// 'recycle' 후보 수정의 순수 LRU 로직. 파킹된 GL 캔버스(고정된 webgl2 컨텍스트를 유지한 채)를
// 재사용 풀에 담아 컨텍스트 churn을 줄인다. DOM/컨텍스트 부작용은 훅으로 주입해 순수하게
// 단위 테스트 가능하도록 분리했다 — hit/miss/evict/lost-eviction를 이 파일 하나로 검증한다.

export type RecyclerHooks<T> = {
  // 컨텍스트가 이미 로스된 항목은 절대 풀에 넣지 않는다(park) / 꺼낼 때 걸러 축출한다(acquire).
  isLost: (item: T) => boolean;
  // 오버플로 축출·로스 축출 시 실제 처분(loseContext + 노드 제거 + 통계)을 수행한다.
  dispose: (item: T) => void;
};

export type ParkResult = 'pooled' | 'lost';

// cap개까지 파킹하는 LRU. 가장 최근에 파킹한 항목을 먼저 재사용(warm)하고, 오버플로 시
// 가장 오래된 항목을 축출한다.
export class GlCanvasRecycler<T> {
  #cap: number;
  #hooks: RecyclerHooks<T>;
  // oldest first, newest last
  #items: T[] = [];

  constructor(cap: number, hooks: RecyclerHooks<T>) {
    this.#cap = Math.max(0, cap);
    this.#hooks = hooks;
  }

  // 로스된 컨텍스트는 풀에 넣지 않는다('lost' 반환 — 호출부가 기존 처분 경로로 폴백). 정상이면
  // 풀에 넣고, cap 초과 시 가장 오래된 항목부터 dispose로 축출한다.
  park(item: T): ParkResult {
    if (this.#cap === 0) return 'lost';
    if (this.#hooks.isLost(item)) return 'lost';
    this.#items.push(item);
    while (this.#items.length > this.#cap) {
      const evicted = this.#items.shift();
      if (evicted !== undefined) this.#hooks.dispose(evicted);
    }
    return 'pooled';
  }

  // 가장 최근 파킹 항목을 반환한다(재사용). 도중에 로스된 항목을 만나면 dispose로 축출하고 계속
  // 찾는다. 비었으면 undefined(호출부가 새로 생성).
  acquire(): T | undefined {
    while (this.#items.length > 0) {
      const item = this.#items.pop();
      if (item === undefined) break;
      if (this.#hooks.isLost(item)) {
        this.#hooks.dispose(item);
        continue;
      }
      return item;
    }
    return undefined;
  }

  // 풀에서 항목만 제거한다(dispose 호출 없음) — 풀에 있는 동안 webglcontextlost가 발화해 호출부가
  // 직접 처분을 마친 뒤 원장만 정리하는 용도.
  drop(item: T): boolean {
    const index = this.#items.indexOf(item);
    if (index === -1) return false;
    this.#items.splice(index, 1);
    return true;
  }

  // 풀에 남은 모든 항목을 dispose로 비운다(에디터 teardown 시 파킹된 GL 컨텍스트 누수 방지).
  flush(): void {
    while (this.#items.length > 0) {
      const item = this.#items.pop();
      if (item !== undefined) this.#hooks.dispose(item);
    }
  }

  size(): number {
    return this.#items.length;
  }
}
