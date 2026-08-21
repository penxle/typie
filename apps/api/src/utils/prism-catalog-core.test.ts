import assert from 'node:assert/strict';
import test from 'node:test';
import { createTtlCache } from './prism-catalog-core.ts';

test('createTtlCache: TTL 안은 재사용, 지나면 다시 읽고, 실패는 캐시하지 않는다', async () => {
  let now = 0;
  let calls = 0;
  let fail = false;
  const get = createTtlCache({
    load: async () => {
      calls += 1;
      if (fail) throw new Error('down');
      return calls;
    },
    ttlMs: 100,
    now: () => now,
  });

  assert.equal(await get(), 1);
  assert.equal(await get(), 1);
  now = 100;
  assert.equal(await get(), 2);
  now = 200;
  fail = true;
  await assert.rejects(get(), /down/);
  fail = false;
  assert.equal(await get(), 4);
  assert.equal(await get(), 4);
});

test('createTtlCache: failureTtlMs 안에서는 같은 실패를 다시 던지고 load를 부르지 않는다', async () => {
  let now = 0;
  let calls = 0;
  let fail = true;
  const get = createTtlCache({
    load: async () => {
      calls += 1;
      if (fail) throw new Error(`down${calls}`);
      return calls;
    },
    ttlMs: 100,
    failureTtlMs: 30,
    now: () => now,
  });

  await assert.rejects(get(), /down1/);
  now = 10;
  await assert.rejects(get(), /down1/);
  assert.equal(calls, 1);
  now = 30;
  fail = false;
  assert.equal(await get(), 2);
  assert.equal(await get(), 2);
});
