import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  FontLoaderState,
  loadOnce,
  PRELOAD_CONCURRENCY,
  PreloadQueue,
  RETRY_BASE_MS,
  RETRY_CAP_MS,
  RETRY_MAX_ATTEMPTS,
  scheduleRetry,
} from './fonts';

describe('FontLoaderState.generationOf', () => {
  it('미등록 폰트 키는 세대 0을 반환한다', () => {
    const state = new FontLoaderState();
    expect(state.generationOf('Pretendard:400')).toBe(0);
  });

  it('파견 시점과 가드 비교가 동일한 헬퍼를 사용해 미등록 상태에서도 stale로 보지 않는다', () => {
    const state = new FontLoaderState();
    const dispatchGen = state.generationOf('Pretendard:400');
    expect(state.isStale('Pretendard:400', dispatchGen)).toBe(false);
  });

  it('동일 hash로 롤백(A→B→A)해도 구 파견 세대는 stale로 판정된다', () => {
    const state = new FontLoaderState();
    const fk = 'Pretendard:400';
    const dispatched = state.generationOf(fk);
    state.purge([fk]);
    state.purge([fk]);
    expect(state.isStale(fk, dispatched)).toBe(true);
  });

  it('관련 없는 폰트 키는 purge에 영향받지 않는다', () => {
    const state = new FontLoaderState();
    state.loaded.add('base:Other:700:h9');
    state.purge(['Pretendard:400']);
    expect(state.loaded.has('base:Other:700:h9')).toBe(true);
  });
});

describe('loadOnce', () => {
  it('커밋되면 loaded에 등록되고 true를 반환한다', async () => {
    const state = new FontLoaderState();
    const key = 'base:Pretendard:400:h1';

    const committed = await loadOnce(state, key, async () => {
      state.loaded.add(key);
      return true;
    });

    expect(committed).toBe(true);
    expect(state.loaded.has(key)).toBe(true);
  });

  it('stale 폐기는 false를 반환하고 loaded를 오염시키지 않는다', async () => {
    const state = new FontLoaderState();
    const key = 'base:Pretendard:400:h1';

    const committed = await loadOnce(state, key, async () => false);

    expect(committed).toBe(false);
    expect(state.loaded.has(key)).toBe(false);
    expect(state.loading.has(key)).toBe(false);
  });

  it('동시 호출은 동일 promise에 합류하고 fn을 한 번만 실행한다', async () => {
    const state = new FontLoaderState();
    const key = 'base:Pretendard:400:h1';
    let calls = 0;
    const { promise: gate, resolve: release }: PromiseWithResolvers<void> = Promise.withResolvers();

    const leader = loadOnce(state, key, async () => {
      calls++;
      await gate;
      return true;
    });
    const waiter = loadOnce(state, key, async () => {
      calls++;
      return true;
    });

    release();
    const [leaderResult, waiterResult] = await Promise.all([leader, waiter]);

    expect(calls).toBe(1);
    expect(leaderResult).toBe(true);
    expect(waiterResult).toBe(true);
  });

  it('구 promise 정리가 신 엔트리를 지우지 않는다(ABA 방지)', async () => {
    const state = new FontLoaderState();
    const key = 'manifest:Pretendard:400:h1';

    const { promise: oldPromise, resolve: resolveOld } = Promise.withResolvers<boolean>();
    const oldLoad = loadOnce(state, key, () => oldPromise);

    expect(state.loading.get(key)).toBe(oldPromise);

    state.purge(['Pretendard:400']);
    expect(state.loading.has(key)).toBe(false);

    const newPromise = Promise.resolve(true);
    state.loading.set(key, newPromise);

    resolveOld(true);
    await oldLoad;

    expect(state.loading.get(key)).toBe(newPromise);
  });
});

describe('scheduleRetry', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('backoff을 base부터 cap까지 진행하다 상한에서 자멸한다', async () => {
    const state = new FontLoaderState();
    const key = 'manifest:Pretendard:400:h1';
    state.loading.set(key, Promise.resolve(false));

    let calls = 0;
    scheduleRetry(state, key, 0, async () => {
      calls++;
    });
    expect(state.retryScheduled.get(key)).toEqual({ gen: 0, attempt: 1 });

    await vi.advanceTimersByTimeAsync(RETRY_BASE_MS);
    expect(state.retryScheduled.get(key)).toEqual({ gen: 0, attempt: 1 });

    await vi.advanceTimersByTimeAsync(RETRY_BASE_MS * 2);
    expect(state.retryScheduled.get(key)).toEqual({ gen: 0, attempt: 2 });

    await vi.advanceTimersByTimeAsync(RETRY_BASE_MS * 4);
    expect(state.retryScheduled.get(key)).toEqual({ gen: 0, attempt: 3 });

    await vi.advanceTimersByTimeAsync(RETRY_BASE_MS * 8);
    expect(state.retryScheduled.get(key)).toEqual({ gen: 0, attempt: 4 });

    await vi.advanceTimersByTimeAsync(RETRY_CAP_MS);

    expect(calls).toBe(0);
    expect(state.retryScheduled.has(key)).toBe(false);
    expect(RETRY_MAX_ATTEMPTS).toBe(5);
  });

  it('소유권을 상실하면(gen 불일치) 자멸하고 새 소유자의 항목은 건드리지 않는다', async () => {
    const state = new FontLoaderState();
    const key = 'manifest:Pretendard:400:h1';

    let calls = 0;
    scheduleRetry(state, key, 0, async () => {
      calls++;
    });

    state.retryScheduled.set(key, { gen: 1, attempt: 1 });

    await vi.advanceTimersByTimeAsync(RETRY_BASE_MS);

    expect(calls).toBe(0);
    expect(state.retryScheduled.get(key)).toEqual({ gen: 1, attempt: 1 });
  });

  it('loading에 합류한 뒤 여전히 실패 상태면 체인을 이어간다', async () => {
    const state = new FontLoaderState();
    const key = 'manifest:Pretendard:400:h1';

    const { promise: inflight, resolve: release } = Promise.withResolvers<boolean>();
    state.loading.set(key, inflight);

    let calls = 0;
    scheduleRetry(state, key, 0, async () => {
      calls++;
    });

    await vi.advanceTimersByTimeAsync(RETRY_BASE_MS);
    expect(state.retryScheduled.get(key)).toEqual({ gen: 0, attempt: 1 });

    release(false);
    await vi.advanceTimersByTimeAsync(0);

    expect(calls).toBe(0);
    expect(state.retryScheduled.get(key)).toEqual({ gen: 0, attempt: 1 });

    await vi.advanceTimersByTimeAsync(RETRY_BASE_MS * 2);
    expect(state.retryScheduled.get(key)).toEqual({ gen: 0, attempt: 2 });
  });

  it('loading에 합류한 뒤 loaded로 커밋되면 체인이 종료된다', async () => {
    const state = new FontLoaderState();
    const key = 'manifest:Pretendard:400:h1';

    const { promise: inflight, resolve: release } = Promise.withResolvers<boolean>();
    state.loading.set(key, inflight);

    let calls = 0;
    scheduleRetry(state, key, 0, async () => {
      calls++;
    });

    await vi.advanceTimersByTimeAsync(RETRY_BASE_MS);

    state.loaded.add(key);
    release(true);
    await vi.advanceTimersByTimeAsync(0);

    expect(calls).toBe(0);
    expect(state.retryScheduled.has(key)).toBe(false);
  });

  it('loading이 비어있으면 재파견이 fn을 직접 호출하고, 실패하면 다음 attempt로 넘어간다', async () => {
    const state = new FontLoaderState();
    const key = 'manifest:Pretendard:400:h1';

    let calls = 0;
    scheduleRetry(state, key, 0, async () => {
      calls++;
      throw new Error('load failed');
    });
    expect(state.retryScheduled.get(key)).toEqual({ gen: 0, attempt: 1 });

    await vi.advanceTimersByTimeAsync(RETRY_BASE_MS);
    expect(calls).toBe(1);
    expect(state.retryScheduled.get(key)).toEqual({ gen: 0, attempt: 1 });

    await vi.advanceTimersByTimeAsync(RETRY_BASE_MS * 2);
    expect(calls).toBe(2);
    expect(state.retryScheduled.get(key)).toEqual({ gen: 0, attempt: 2 });
  });

  it('loading이 비어있으면 재파견이 fn을 직접 호출하고, 성공하면 체인이 즉시 종료된다', async () => {
    const state = new FontLoaderState();
    const key = 'manifest:Pretendard:400:h1';

    let calls = 0;
    scheduleRetry(state, key, 0, async () => {
      calls++;
      state.loaded.add(key);
    });
    expect(state.retryScheduled.get(key)).toEqual({ gen: 0, attempt: 1 });

    await vi.advanceTimersByTimeAsync(RETRY_BASE_MS);
    expect(calls).toBe(1);
    expect(state.retryScheduled.has(key)).toBe(false);

    await vi.advanceTimersByTimeAsync(RETRY_BASE_MS * 2);
    expect(calls).toBe(1);
    expect(state.retryScheduled.has(key)).toBe(false);
  });
});

describe('PreloadQueue.purge', () => {
  it('아직 실행되지 않은 pending 항목을 제거할 때 promise를 settle한다(무기한 hang 방지)', async () => {
    const queue = new PreloadQueue();
    const hang = (): Promise<void> =>
      new Promise((resolve) => {
        void resolve;
      });

    for (let i = 0; i < PRELOAD_CONCURRENCY; i++) {
      queue.enqueue(`chunk:Pretendard:400:h1:${i}`, i, hang);
    }
    const pending = queue.enqueue(`chunk:Pretendard:400:h1:${PRELOAD_CONCURRENCY}`, PRELOAD_CONCURRENCY, hang);

    queue.purge(['chunk:Pretendard:400:h1:']);

    await expect(pending).resolves.toBeUndefined();
  });
});

describe('PreloadQueue 드레인 순서', () => {
  it('priority 오름차순(manifest → base → 청크 빈도순)으로 드레인한다', async () => {
    const queue = new PreloadQueue();
    const started: string[] = [];
    const blockers: (() => void)[] = [];
    const blocked = (key: string) => () => {
      started.push(key);
      return new Promise<void>((resolve) => {
        blockers.push(resolve);
      });
    };
    const flush = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

    for (let i = 0; i < 4; i++) {
      void queue.enqueue(`order:fill:${i}`, 100, blocked(`order:fill:${i}`));
    }
    void queue.enqueue('order:chunk:9', 9, blocked('order:chunk:9'));
    void queue.enqueue('order:chunk:2', 2, blocked('order:chunk:2'));
    void queue.enqueue('order:manifest', -2, blocked('order:manifest'));
    void queue.enqueue('order:base', -1, blocked('order:base'));

    blockers[0]();
    await flush();
    expect(started[4]).toBe('order:manifest');
    blockers[4]();
    await flush();
    expect(started[5]).toBe('order:base');
    blockers[5]();
    await flush();
    expect(started[6]).toBe('order:chunk:2');
    blockers[6]();
    await flush();
    expect(started[7]).toBe('order:chunk:9');

    for (const release of blockers) release();
    await flush();
  });
});
