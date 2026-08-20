import assert from 'node:assert/strict';
import test from 'node:test';
import { activeRun, createPrismClient, mapPrismError, PrismApiError, sessionTitleFrom } from './prism-core.ts';

type Init = { method?: string; body?: unknown; headers?: Record<string, string> };
const fakeHttp = (handler: (path: string, init?: Init) => { status: number; json?: unknown; stream?: string }) => {
  const calls: { path: string; init?: Init }[] = [];
  return {
    calls,
    request: async (path: string, init?: Init) => {
      calls.push({ path, init });
      const res = handler(path, init);
      const body =
        res.stream === undefined
          ? null
          : new ReadableStream<Uint8Array>({
              start(c) {
                c.enqueue(new TextEncoder().encode(res.stream ?? ''));
                c.close();
              },
            });
      return { status: res.status, json: async () => res.json ?? {}, body };
    },
  };
};

test('sessionTitleFrom: 개행을 공백으로, 40자 절단, 빈 문자열은 null', () => {
  assert.equal(sessionTitleFrom('  안녕\n하세요  '), '안녕 하세요');
  assert.equal(sessionTitleFrom('가'.repeat(50)), '가'.repeat(40));
  assert.equal(sessionTitleFrom(' '.repeat(3)), null);
});

test('activeRun: running인 마지막 run', () => {
  assert.deepEqual(
    activeRun([
      { runSeq: 1, status: 'completed' },
      { runSeq: 2, status: 'running' },
    ]),
    { runSeq: 2, status: 'running' },
  );
  assert.equal(activeRun([{ runSeq: 1, status: 'failed' }]), null);
});

test('mapPrismError: 본문 코드 우선, 없으면 internal', () => {
  assert.equal(mapPrismError(409, { error: 'run-active' }).code, 'run-active');
  assert.equal(mapPrismError(500, 'nope').code, 'internal');
  assert.ok(mapPrismError(404, { error: 'not-found' }) instanceof PrismApiError);
});

test('invokeAgent는 POST /agents에 app·agent·metadata를 싣고 runSeq를 돌려준다', async () => {
  const http = fakeHttp(() => ({ status: 200, json: { runSeq: 1 } }));
  const result = await createPrismClient(http).invokeAgent({ agentId: 'typie-x', message: '안녕', key: 'k1', metadata: { userId: 'U1' } });
  assert.deepEqual(result, { runSeq: 1 });
  assert.equal(http.calls[0].path, '/agents');
  assert.deepEqual(http.calls[0].init?.body, {
    agentId: 'typie-x',
    app: 'assistant',
    agent: 'chat',
    message: '안녕',
    key: 'k1',
    metadata: { userId: 'U1' },
  });
});

test('비2xx는 PrismApiError로 던진다', async () => {
  const client = createPrismClient(fakeHttp(() => ({ status: 409, json: { error: 'run-active' } })));
  await assert.rejects(
    client.resumeAgent('typie-x', { message: 'a', key: 'k' }),
    (err: unknown) => err instanceof PrismApiError && err.code === 'run-active' && err.status === 409,
  );
});

test('getAgent는 runs·pending·invocations만 남기고, 형태가 어긋나면 malformed-response', async () => {
  const ok = await createPrismClient(
    fakeHttp(() => ({
      status: 200,
      json: { agent: { id: 'x' }, runs: [{ runSeq: 1, status: 'running', input: 'm' }], pending: null, invocations: [] },
    })),
  ).getAgent('typie-x');
  assert.deepEqual(ok, { runs: [{ runSeq: 1, status: 'running' }], pending: null, invocations: [] });
  await assert.rejects(
    createPrismClient(fakeHttp(() => ({ status: 200, json: { runs: 'nope' } }))).getAgent('typie-x'),
    (err: unknown) => err instanceof PrismApiError && err.code === 'malformed-response',
  );
});

test('openAgentEvents는 Last-Event-ID 헤더로 커서를 싣고, readAgentEventsUntilSync는 sync까지 모은다', async () => {
  const stream =
    'id: 1\nevent: run.started\ndata: {"seq":1,"kind":"run.started","occurredAt":1,"loggedAt":1,"context":{"agent":{"id":"a","name":"x"},"run":1},"data":{"message":"m","key":null,"tag":null}}\n\nevent: sync\ndata: {"seq":1}\n\n';
  const http = fakeHttp(() => ({ status: 200, stream }));
  const result = await createPrismClient(http).readAgentEventsUntilSync('typie-x', 7, new AbortController().signal);
  assert.equal(http.calls[0].path, '/agents/typie-x/events');
  assert.equal(http.calls[0].init?.headers?.['last-event-id'], '7');
  assert.equal(result.sync, 1);
  assert.equal(result.events[0].kind, 'run.started');
});
