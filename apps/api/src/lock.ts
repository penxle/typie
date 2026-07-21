import dedent from 'dedent';
import { nanoid } from 'nanoid';
import { redis } from '#/cache.ts';

export class Lock {
  #id: string;

  #lockKey: string;
  #waitKey: string;

  #ttlSeconds: number;

  #acquired = false;
  #timer?: NodeJS.Timeout;
  #controller: AbortController;

  constructor(key: string, options: { ttlSeconds?: number } = {}) {
    this.#id = nanoid();

    this.#lockKey = `lock:${key}`;
    this.#waitKey = `lock:wait:${key}`;

    this.#ttlSeconds = options.ttlSeconds ?? 30;

    this.#controller = new AbortController();
  }

  #start() {
    if (!this.#acquired) return;

    this.#timer = setInterval(async () => {
      try {
        const renewed = await this.#extend();
        if (!renewed) {
          this.#stop();
          this.#acquired = false;
          this.#controller.abort();
        }
      } catch {
        this.#stop();
        this.#acquired = false;
        this.#controller.abort();
      }
    }, 10_000);

    this.#timer.unref();
  }

  #stop() {
    if (!this.#timer) {
      return;
    }

    clearInterval(this.#timer);
    this.#timer = undefined;
  }

  async #extend() {
    if (!this.#acquired) return false;

    const script = dedent`
      if redis.call("get", KEYS[1]) == ARGV[1] then
        return redis.call("expire", KEYS[1], ARGV[2])
      else
        return 0
      end
    `;

    const result = await redis.eval(script, 1, this.#lockKey, this.#id, this.#ttlSeconds);
    return result === 1;
  }

  get signal(): AbortSignal {
    return this.#controller.signal;
  }

  async acquire() {
    const deadline = Date.now() + 30_000;

    while (Date.now() < deadline) {
      const acquired = await redis.set(this.#lockKey, this.#id, 'EX', this.#ttlSeconds, 'NX');
      if (acquired === 'OK') {
        this.#acquired = true;
        this.#start();
        return true;
      }

      const remainingTime = Math.ceil((deadline - Date.now()) / 1000);
      if (remainingTime <= 0) break;

      await redis.blpop(this.#waitKey, Math.min(remainingTime, 1));
    }

    return false;
  }

  async tryAcquire() {
    const acquired = await redis.set(this.#lockKey, this.#id, 'EX', this.#ttlSeconds, 'NX');
    if (acquired === 'OK') {
      this.#acquired = true;
      this.#start();
      return true;
    }
    return false;
  }

  async release() {
    if (!this.#acquired) return false;

    this.#stop();
    this.#controller.abort();

    const script = dedent`
      if redis.call("get", KEYS[1]) == ARGV[1] then
        redis.call("del", KEYS[1])
        redis.call("rpush", KEYS[2], "1")
        redis.call("expire", KEYS[2], 30)
        return 1
      else
        return 0
      end
    `;

    const result = await redis.eval(script, 2, this.#lockKey, this.#waitKey, this.#id);

    if (result === 1) {
      this.#acquired = false;
      return true;
    }

    return false;
  }
}

export const withLock = async <T>(key: string, fn: (signal: AbortSignal) => Promise<T>) => {
  const lock = new Lock(key);

  const acquired = await lock.acquire();
  if (!acquired) {
    throw new Error(`Failed to acquire lock for key: ${key}`);
  }

  try {
    return await fn(lock.signal);
  } finally {
    await lock.release();
  }
};
