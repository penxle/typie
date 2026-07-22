import { Worker } from 'node:worker_threads';
import type { CollectResult, ConsolidateResult } from '@typie/editor-ffi/server';

const MAX_CONSECUTIVE_FAILURES = 3;
const COOLDOWN_MS = 60_000;
const BACKOFF_BASE_MS = 1000;
const DEFAULT_CALL_TIMEOUT_MS = 600_000;
const DEFAULT_READY_TIMEOUT_MS = 30_000;
const RECLAIM_RETRY_MS = 1000;

type CallResult = { result: unknown; execMs: number };

type WorkerMessage = {
  id: number;
  ok: boolean;
  result?: unknown;
  execMs?: number;
  poisoned?: boolean;
  error?: { name: string; message: string; stack?: string };
};

export type WorkerLike = {
  on(event: 'message', listener: (msg: WorkerMessage) => void): void;
  on(event: 'error', listener: (err: Error) => void): void;
  on(event: 'exit', listener: (code: number) => void): void;
  postMessage(value: unknown): void;
  terminate(): unknown;
};

export type WasmThreadOptions = {
  callTimeoutMs?: number;
  readyTimeoutMs?: number;
  backoffBaseMs?: number;
  cooldownMs?: number;
  maxConsecutiveFailures?: number;
};

type Pending = {
  resolve: (value: CallResult) => void;
  reject: (err: Error) => void;
  timer: NodeJS.Timeout;
};

export type Thread = {
  readonly healthy: boolean;
  call(method: string, args: Uint8Array[], timeoutMs?: number): Promise<CallResult>;
  waitHealthy(): Promise<void>;
  shutdown(): void;
};

export class WasmThread implements Thread {
  #factory: () => WorkerLike;
  #callTimeoutMs: number;
  #readyTimeoutMs: number;
  #backoffBaseMs: number;
  #cooldownMs: number;
  #maxFailures: number;

  #worker: WorkerLike | null = null;
  #pending = new Map<number, Pending>();
  #nextId = 0;
  #generation = 0;
  #ready!: Promise<void>;
  #readyResolved = false;
  #initTimer: NodeJS.Timeout | null = null;
  #backoffTimer: NodeJS.Timeout | null = null;
  #consecutiveFailures = 0;
  #unhealthy = false;
  #cooldownUntil = 0;
  #shuttingDown = false;

  constructor(factory: () => WorkerLike, options: WasmThreadOptions = {}) {
    this.#factory = factory;
    this.#callTimeoutMs = options.callTimeoutMs ?? DEFAULT_CALL_TIMEOUT_MS;
    this.#readyTimeoutMs = options.readyTimeoutMs ?? DEFAULT_READY_TIMEOUT_MS;
    this.#backoffBaseMs = options.backoffBaseMs ?? BACKOFF_BASE_MS;
    this.#cooldownMs = options.cooldownMs ?? COOLDOWN_MS;
    this.#maxFailures = options.maxConsecutiveFailures ?? MAX_CONSECUTIVE_FAILURES;
    this.#spawn();
  }

  #spawn(): void {
    if (this.#shuttingDown) {
      return;
    }

    const gen = ++this.#generation;
    this.#readyResolved = false;

    let worker: WorkerLike;
    try {
      worker = this.#factory();
    } catch (err) {
      this.#ready = Promise.reject(err as Error);
      this.#ready.catch(() => false);
      this.#onInitFailure(gen, err as Error);
      return;
    }
    this.#worker = worker;

    this.#ready = new Promise<void>((resolve, reject) => {
      this.#initTimer = setTimeout(() => {
        if (gen !== this.#generation) {
          return;
        }
        reject(new Error(`wasm thread init timed out after ${this.#readyTimeoutMs}ms`));
      }, this.#readyTimeoutMs);
      this.#initTimer.unref?.();

      worker.on('message', (msg) => {
        if (gen !== this.#generation) {
          return;
        }
        if (msg.id === -1) {
          this.#clearInitTimer();
          this.#readyResolved = true;
          this.#consecutiveFailures = 0;
          resolve();
          return;
        }
        const pending = this.#pending.get(msg.id);
        if (!pending) {
          return;
        }
        clearTimeout(pending.timer);
        this.#pending.delete(msg.id);
        if (msg.ok) {
          pending.resolve({ result: msg.result, execMs: msg.execMs ?? 0 });
        } else {
          pending.reject(this.#toError(msg.error));
          if (msg.poisoned) {
            this.#teardown(gen, new Error('wasm thread poisoned'));
          }
        }
      });

      worker.on('error', (err) => {
        if (gen !== this.#generation) {
          return;
        }
        this.#clearInitTimer();
        if (this.#readyResolved) {
          this.#teardown(gen, err);
        } else {
          reject(err);
        }
      });

      worker.on('exit', (code) => {
        if (gen !== this.#generation) {
          return;
        }
        this.#clearInitTimer();
        const err = new Error(`wasm thread exited with code ${code}`);
        if (this.#readyResolved) {
          this.#teardown(gen, err);
        } else {
          reject(err);
        }
      });
    });

    this.#ready.catch((err) => {
      if (gen !== this.#generation || this.#readyResolved) {
        return;
      }
      this.#onInitFailure(gen, err as Error);
    });
  }

  #onInitFailure(gen: number, err: Error): void {
    if (gen !== this.#generation) {
      return;
    }
    this.#generation++;
    this.#clearInitTimer();
    const dying = this.#worker;
    this.#worker = null;
    this.#rejectAllPending(err);
    this.#safeTerminate(dying);
    if (this.#shuttingDown) {
      return;
    }

    this.#consecutiveFailures++;
    if (this.#consecutiveFailures >= this.#maxFailures) {
      this.#unhealthy = true;
      this.#cooldownUntil = Date.now() + this.#cooldownMs;
      this.#ready = Promise.reject(new Error('wasm thread unhealthy'));
      this.#ready.catch(() => false);
      return;
    }

    const delay = this.#backoffBaseMs * 2 ** (this.#consecutiveFailures - 1);
    this.#backoffTimer = setTimeout(() => this.#spawn(), delay);
    this.#backoffTimer.unref?.();
  }

  #teardown(gen: number, err: Error): void {
    if (gen !== this.#generation) {
      return;
    }
    this.#generation++;
    this.#clearInitTimer();
    const dying = this.#worker;
    this.#worker = null;
    this.#readyResolved = false;
    this.#rejectAllPending(err);
    this.#safeTerminate(dying);
    if (this.#shuttingDown) {
      return;
    }
    this.#spawn();
  }

  #rejectAllPending(err: Error): void {
    for (const pending of this.#pending.values()) {
      clearTimeout(pending.timer);
      pending.reject(err);
    }
    this.#pending.clear();
  }

  #clearInitTimer(): void {
    if (this.#initTimer) {
      clearTimeout(this.#initTimer);
      this.#initTimer = null;
    }
  }

  #safeTerminate(worker: WorkerLike | null): void {
    if (!worker) {
      return;
    }
    try {
      void worker.terminate();
    } catch {
      /* ignore */
    }
  }

  #toError(error?: { name: string; message: string; stack?: string }): Error {
    const err = new Error(error?.message ?? 'wasm thread error');
    return error ? Object.assign(err, error) : err;
  }

  #retryFromCooldown(): boolean {
    if (!this.#unhealthy) {
      return true;
    }
    if (Date.now() < this.#cooldownUntil) {
      return false;
    }
    this.#unhealthy = false;
    this.#consecutiveFailures = 0;
    this.#spawn();
    return true;
  }

  async call(method: string, args: Uint8Array[], timeoutMs: number = this.#callTimeoutMs): Promise<CallResult> {
    if (this.#shuttingDown) {
      throw new Error('wasm thread pool is shutting down');
    }
    if (this.#unhealthy && !this.#retryFromCooldown()) {
      throw new Error('wasm thread unhealthy; retry after cooldown');
    }

    await this.#ready;

    const gen = this.#generation;
    const worker = this.#worker;
    if (!worker) {
      throw new Error('wasm thread unavailable');
    }

    const id = this.#nextId++;
    return new Promise<CallResult>((resolve, reject) => {
      const timer = setTimeout(() => {
        if (gen !== this.#generation) {
          return;
        }
        this.#teardown(gen, new Error(`wasm thread call timed out after ${timeoutMs}ms`));
      }, timeoutMs);
      timer.unref?.();

      this.#pending.set(id, { resolve, reject, timer });

      try {
        worker.postMessage({ id, method, args });
      } catch (err) {
        clearTimeout(timer);
        this.#pending.delete(id);
        reject(err as Error);
      }
    });
  }

  async waitHealthy(): Promise<void> {
    if (this.#shuttingDown) {
      throw new Error('wasm thread pool is shutting down');
    }
    if (this.#unhealthy && !this.#retryFromCooldown()) {
      throw new Error('wasm thread unhealthy; retry after cooldown');
    }
    await this.#ready;
  }

  shutdown(): void {
    this.#shuttingDown = true;
    this.#clearInitTimer();
    if (this.#backoffTimer) {
      clearTimeout(this.#backoffTimer);
      this.#backoffTimer = null;
    }
    this.#generation++;
    this.#rejectAllPending(new Error('wasm thread shut down'));
    const dying = this.#worker;
    this.#worker = null;
    this.#safeTerminate(dying);
  }

  get healthy(): boolean {
    return this.#readyResolved && !this.#unhealthy && !this.#shuttingDown;
  }

  get pendingSize(): number {
    return this.#pending.size;
  }
}

export const createPool = (makeThread: () => Thread, size: number) => {
  const threads: Thread[] = [];
  const available: Thread[] = [];
  const waiting: ((thread: Thread) => void)[] = [];
  let started = false;

  const start = () => {
    if (started) {
      return;
    }
    started = true;
    for (let i = 0; i < size; i++) {
      const thread = makeThread();
      threads.push(thread);
      available.push(thread);
    }
  };

  const handOut = (thread: Thread) => {
    const next = waiting.shift();
    if (next) {
      next(thread);
    } else {
      available.push(thread);
    }
  };

  const reclaim = (thread: Thread) => {
    thread.waitHealthy().then(
      () => handOut(thread),
      () => {
        const timer = setTimeout(() => reclaim(thread), RECLAIM_RETRY_MS);
        timer.unref?.();
      },
    );
  };

  const release = (thread: Thread) => {
    if (thread.healthy) {
      handOut(thread);
    } else {
      reclaim(thread);
    }
  };

  const withThread = async <T>(fn: (thread: Thread) => Promise<T>): Promise<T> => {
    start();
    const thread =
      available.pop() ??
      (await new Promise<Thread>((resolve) => {
        waiting.push(resolve);
      }));
    try {
      const result = await fn(thread);
      release(thread);
      return result;
    } catch (err) {
      release(thread);
      throw err;
    }
  };

  const shutdown = () => {
    for (const thread of threads) {
      thread.shutdown();
    }
    threads.length = 0;
    available.length = 0;
    waiting.length = 0;
    started = false;
  };

  return { withThread, shutdown };
};

const workerUrl = new URL('wasm-thread-worker.ts', import.meta.url);

let defaultPool: ReturnType<typeof createPool> | null = null;

const getPool = () => {
  if (!defaultPool) {
    const size = Number(process.env.WASM_THREAD_POOL_SIZE ?? 2);
    if (!Number.isSafeInteger(size) || size <= 0) {
      throw new Error(`invalid WASM_THREAD_POOL_SIZE: ${process.env.WASM_THREAD_POOL_SIZE}`);
    }
    defaultPool = createPool(() => new WasmThread(() => new Worker(workerUrl)), size);
  }
  return defaultPool;
};

export const wasmThread = {
  collectFold: (existing: Uint8Array, packed: Uint8Array) =>
    getPool().withThread((thread) => thread.call('collect_fold', [existing, packed])) as Promise<{ result: CollectResult; execMs: number }>,
  consolidate: (stream: Uint8Array) =>
    getPool().withThread((thread) => thread.call('consolidate', [stream])) as Promise<{ result: ConsolidateResult; execMs: number }>,
  extractProse: (graph: Uint8Array) =>
    getPool().withThread((thread) => thread.call('extract_prose', [graph])) as Promise<{ result: string | null; execMs: number }>,
};

export const shutdownWasmThreadPool = () => {
  defaultPool?.shutdown();
  defaultPool = null;
};
