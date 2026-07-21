import assert from 'node:assert/strict';
import test from 'node:test';
import { createPool, WasmThread } from './wasm-thread.ts';
import type { Thread, WorkerLike } from './wasm-thread.ts';

type Emitted = {
  id: number;
  ok: boolean;
  result?: unknown;
  execMs?: number;
  poisoned?: boolean;
  error?: { name: string; message: string; stack?: string };
};

class FakeWorker {
  #message: ((msg: Emitted) => void)[] = [];
  #error: ((err: Error) => void)[] = [];
  #exit: ((code: number) => void)[] = [];

  posted: { id: number; method: string; args: Uint8Array[] }[] = [];
  terminated = false;
  throwOnPost = false;
  emitExitOnTerminate = false;

  on(event: 'message' | 'error' | 'exit', listener: (arg: never) => void): void {
    if (event === 'message') {
      this.#message.push(listener as (msg: Emitted) => void);
    } else if (event === 'error') {
      this.#error.push(listener as (err: Error) => void);
    } else {
      this.#exit.push(listener as (code: number) => void);
    }
  }

  postMessage(value: unknown): void {
    if (this.throwOnPost) {
      throw new Error('postMessage failed');
    }
    this.posted.push(value as { id: number; method: string; args: Uint8Array[] });
  }

  terminate(): unknown {
    this.terminated = true;
    if (this.emitExitOnTerminate) {
      this.emitExit(1);
    }
    return Promise.resolve(0);
  }

  emitMessage(msg: Emitted): void {
    for (const cb of this.#message) {
      cb(msg);
    }
  }

  emitError(err: Error): void {
    for (const cb of this.#error) {
      cb(err);
    }
  }

  emitExit(code: number): void {
    for (const cb of this.#exit) {
      cb(code);
    }
  }

  ready(): void {
    this.emitMessage({ id: -1, ok: true });
  }

  poison(id: number): void {
    this.emitMessage({ id, ok: false, poisoned: true, error: { name: 'RuntimeError', message: 'poisoned' } });
  }

  result(id: number, result: unknown): void {
    this.emitMessage({ id, ok: true, result, execMs: 1 });
  }

  lastPostedId(): number {
    const last = this.posted.at(-1);
    if (!last) {
      throw new Error('no posted message');
    }
    return last.id;
  }
}

const makeFactory = () => {
  const workers: FakeWorker[] = [];
  const factory = (): WorkerLike => {
    const worker = new FakeWorker();
    workers.push(worker);
    return worker as unknown as WorkerLike;
  };
  return { workers, factory };
};

const flush = () => new Promise<void>((resolve) => setImmediate(resolve));
const delay = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

const until = async (cond: () => boolean, timeoutMs = 1000) => {
  const start = Date.now();
  while (!cond()) {
    if (Date.now() - start > timeoutMs) {
      throw new Error('until: timed out');
    }
    await delay(1);
  }
};

test('spawn 실패 시 call은 hang 없이 reject된다', async () => {
  const { workers, factory } = makeFactory();
  const thread = new WasmThread(factory, { backoffBaseMs: 5, readyTimeoutMs: 200 });
  await flush();
  workers[0].emitError(new Error('boom'));
  await assert.rejects(thread.call('collect_fold', [new Uint8Array()]), /boom/);
  thread.shutdown();
});

test('연속 3회 실패 시 unhealthy로 즉시 reject하고 쿨다운 후 재시도한다', async () => {
  const { workers, factory } = makeFactory();
  const thread = new WasmThread(factory, { backoffBaseMs: 2, cooldownMs: 40, maxConsecutiveFailures: 3, readyTimeoutMs: 200 });

  await until(() => workers.length === 1);
  workers[0].emitError(new Error('f1'));
  await until(() => workers.length === 2);
  workers[1].emitError(new Error('f2'));
  await until(() => workers.length === 3);
  workers[2].emitError(new Error('f3'));
  await flush();

  assert.equal(thread.healthy, false);
  await assert.rejects(thread.call('consolidate', [new Uint8Array()]), /unhealthy/);

  await delay(60);
  const callP = thread.call('consolidate', [new Uint8Array()]);
  await until(() => workers.length === 4);
  workers[3].ready();
  await flush();
  workers[3].result(workers[3].lastPostedId(), { payload: null });

  const res = await callP;
  assert.deepEqual(res.result, { payload: null });
  thread.shutdown();
});

test('poison 메시지는 해당 pending과 나머지 pending을 모두 reject하고 재생성한다', async () => {
  const { workers, factory } = makeFactory();
  const thread = new WasmThread(factory, { readyTimeoutMs: 200 });

  await until(() => workers.length === 1);
  workers[0].ready();
  await thread.waitHealthy();

  const a = thread.call('collect_fold', [new Uint8Array()]);
  const b = thread.call('consolidate', [new Uint8Array()]);
  await flush();

  workers[0].poison(workers[0].posted[0].id);
  await assert.rejects(a, /poisoned/);
  await assert.rejects(b, /poisoned/);
  assert.equal(workers[0].terminated, true);

  await until(() => workers.length === 2);
  workers[1].ready();
  await thread.waitHealthy();
  assert.equal(thread.healthy, true);
  thread.shutdown();
});

test('terminate가 유발한 exit는 새 세대를 파괴하지 않는다', async () => {
  const { workers, factory } = makeFactory();
  const thread = new WasmThread(factory, { readyTimeoutMs: 200 });

  await until(() => workers.length === 1);
  workers[0].emitExitOnTerminate = true;
  workers[0].ready();
  await thread.waitHealthy();

  const a = thread.call('collect_fold', [new Uint8Array()]);
  await flush();
  workers[0].poison(workers[0].posted[0].id);
  await assert.rejects(a, /poisoned/);

  await until(() => workers.length === 2);
  workers[1].ready();
  await thread.waitHealthy();
  assert.equal(thread.healthy, true);
  assert.equal(workers[1].terminated, false);

  const b = thread.call('consolidate', [new Uint8Array()]);
  await flush();
  workers[1].result(workers[1].lastPostedId(), { payload: null });
  const resB = await b;
  assert.deepEqual(resB.result, { payload: null });
  thread.shutdown();
});

test('실행 데드라인 초과 시 teardown되고 call이 reject된다', async () => {
  const { workers, factory } = makeFactory();
  const thread = new WasmThread(factory, { readyTimeoutMs: 500 });

  await until(() => workers.length === 1);
  workers[0].ready();
  await thread.waitHealthy();

  const a = thread.call('collect_fold', [new Uint8Array()], 20);
  await assert.rejects(a, /timed out/);
  assert.equal(workers[0].terminated, true);
  await until(() => workers.length === 2);
  thread.shutdown();
});

test('정상 완료는 자기 데드라인 타이머를 clear해 현 세대를 유지한다', async () => {
  const { workers, factory } = makeFactory();
  const thread = new WasmThread(factory, { readyTimeoutMs: 500 });

  await until(() => workers.length === 1);
  workers[0].ready();
  await thread.waitHealthy();

  const a = thread.call('collect_fold', [new Uint8Array()], 40);
  await flush();
  workers[0].result(workers[0].lastPostedId(), { ok: 1 });
  const resA = await a;
  assert.deepEqual(resA.result, { ok: 1 });

  await delay(60);
  assert.equal(thread.healthy, true);
  assert.equal(workers[0].terminated, false);
  assert.equal(workers.length, 1);
  thread.shutdown();
});

test('postMessage가 throw하면 pending을 정리하고 reject한다', async () => {
  const { workers, factory } = makeFactory();
  const thread = new WasmThread(factory, { readyTimeoutMs: 200 });

  await until(() => workers.length === 1);
  workers[0].ready();
  await thread.waitHealthy();

  workers[0].throwOnPost = true;
  await assert.rejects(thread.call('collect_fold', [new Uint8Array()]), /postMessage failed/);
  assert.equal(thread.pendingSize, 0);
  assert.equal(thread.healthy, true);
  assert.equal(workers[0].terminated, false);

  workers[0].throwOnPost = false;
  const b = thread.call('consolidate', [new Uint8Array()]);
  await flush();
  workers[0].result(workers[0].lastPostedId(), { payload: null });
  const resB = await b;
  assert.deepEqual(resB.result, { payload: null });
  thread.shutdown();
});

test('혼합 풀에서 unhealthy 스레드는 available로 돌아오지 않는다', async () => {
  const makeFakeThread = (id: string) => {
    const thread = {
      id,
      healthy: true,
      waitHealthyCalls: 0,
      call: async (): Promise<{ result: unknown; execMs: number }> => ({ result: null, execMs: 0 }),
      waitHealthy: (): Promise<void> => {
        thread.waitHealthyCalls++;
        return new Promise<void>(() => false);
      },
      shutdown: () => false,
    };
    return thread;
  };

  const t0 = makeFakeThread('t0');
  const t1 = makeFakeThread('t1');
  const created: Thread[] = [t0, t1];
  let index = 0;
  const pool = createPool(() => created[index++], 2);

  const handedOut: string[] = [];
  await pool.withThread(async (thread) => {
    const t = thread as typeof t1;
    handedOut.push(t.id);
    t.healthy = false;
  });
  await pool.withThread(async (thread) => {
    handedOut.push((thread as typeof t0).id);
  });
  await pool.withThread(async (thread) => {
    handedOut.push((thread as typeof t0).id);
  });

  assert.deepEqual(handedOut, ['t1', 't0', 't0']);
  assert.equal(t1.waitHealthyCalls, 1);
  pool.shutdown();
});
