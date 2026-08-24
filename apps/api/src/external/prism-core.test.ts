import assert from 'node:assert/strict';
import test from 'node:test';
import { activeRun, createPrismClient, mapPrismError, PrismApiError } from './prism-core.ts';

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

test('openAgentEvents는 Last-Event-ID 헤더로 커서를 싣는다', async () => {
  const http = fakeHttp(() => ({ status: 200, stream: 'event: sync\ndata: {"seq":1}\n\n' }));
  await createPrismClient(http).openAgentEvents('typie-x', 7, new AbortController().signal);
  assert.equal(http.calls[0].path, '/agents/typie-x/events');
  assert.equal(http.calls[0].init?.headers?.['last-event-id'], '7');
});

test('writeAgentFiles는 PUT /agents/:id/files에 files를 싣고 written을 돌려준다', async () => {
  const http = fakeHttp(() => ({ status: 200, json: { written: [{ path: 'manuscript/v1.txt', bytes: 3, sha256: 'abc' }] } }));
  const result = await createPrismClient(http).writeAgentFiles('typie-x', [{ path: 'manuscript/v1.txt', content: '원고' }]);
  assert.deepEqual(result.written, [{ path: 'manuscript/v1.txt', bytes: 3, sha256: 'abc' }]);
  assert.equal(http.calls[0].path, '/agents/typie-x/files');
  assert.equal(http.calls[0].init?.method, 'PUT');
  assert.deepEqual(http.calls[0].init?.body, { files: [{ path: 'manuscript/v1.txt', content: '원고' }] });
});

test('resolveTool은 두 세그먼트를 인코딩해 POST …/tools/:toolCallId/result {result}를 보낸다', async () => {
  const http = fakeHttp(() => ({ status: 200, json: {} }));
  await createPrismClient(http).resolveTool('typie-x', 'call/1', { decision: 'declined' });
  assert.equal(http.calls[0].path, '/agents/typie-x/tools/call%2F1/result');
  assert.equal(http.calls[0].init?.method, 'POST');
  assert.deepEqual(http.calls[0].init?.body, { result: { decision: 'declined' } });
});

test('getWorkflow는 GET /workflows/:id를 WorkflowState로 좁히고, cancelWorkflow는 POST …/cancel', async () => {
  const http = fakeHttp((path) =>
    path.endsWith('/cancel')
      ? { status: 200, json: {} }
      : {
          status: 200,
          json: {
            workflow: {
              id: 'workflow_1',
              app: 'app_1',
              workflow: 'high',
              ref: 'PRRR1',
              status: 'running',
              result: null,
              error: null,
              usage: null,
              startedAt: 1,
              finishedAt: null,
              input: '{}',
              caller: 'x',
            },
            invocations: [
              { invocationId: 'i1', targetKind: 'agent', targetId: 'agent_1', originRunSeq: null, status: 'running', step: 'judgment-1' },
            ],
            steps: [],
          },
        },
  );
  const client = createPrismClient(http);
  const state = await client.getWorkflow('workflow_1');
  assert.equal(http.calls[0].path, '/workflows/workflow_1');
  assert.equal(http.calls[0].init?.method, undefined);
  assert.equal(state.workflow.app, 'app_1');
  assert.equal(state.workflow.workflow, 'high');
  assert.equal(state.workflow.ref, 'PRRR1');
  assert.deepEqual(state.invocations, [
    { invocationId: 'i1', targetKind: 'agent', targetId: 'agent_1', originRunSeq: null, status: 'running' },
  ]);
  await client.cancelWorkflow('workflow_1');
  assert.equal(http.calls[1].path, '/workflows/workflow_1/cancel');
  assert.equal(http.calls[1].init?.method, 'POST');
});

test('openWorkflowEvents는 Last-Event-ID 헤더로 워크플로 로그를 연다', async () => {
  const http = fakeHttp(() => ({ status: 200, stream: 'event: sync\ndata: {"seq":0}\n\n' }));
  await createPrismClient(http).openWorkflowEvents('workflow_1', 5, new AbortController().signal);
  assert.equal(http.calls[0].path, '/workflows/workflow_1/events');
  assert.equal(http.calls[0].init?.headers?.['last-event-id'], '5');
});

test('getCatalog는 chat agent의 commands를 배열로 펴고, commands 키가 없으면 null', async () => {
  const http = fakeHttp(() => ({
    status: 200,
    json: {
      models: {},
      agents: {
        chat: { provider: 'p', model: 'm', effort: null, commands: { 리뷰: { description: '리뷰를 시작해요', argumentHint: null } } },
      },
      workflows: {},
    },
  }));
  assert.deepEqual(await createPrismClient(http).getCatalog(), {
    commands: [{ name: '리뷰', description: '리뷰를 시작해요', argumentHint: null }],
  });
  assert.equal(http.calls[0].path, '/apps/assistant/catalog');
  assert.equal(http.calls[0].init?.method, undefined);

  const old = createPrismClient(
    fakeHttp(() => ({ status: 200, json: { models: {}, agents: { chat: { provider: 'p', model: 'm', effort: null } }, workflows: {} } })),
  );
  assert.deepEqual(await old.getCatalog(), { commands: null });

  assert.deepEqual(await createPrismClient(fakeHttp(() => ({ status: 200, json: { agents: {} } }))).getCatalog(), { commands: null });

  assert.deepEqual(await createPrismClient(fakeHttp(() => ({ status: 200, json: { agents: { chat: { commands: {} } } } }))).getCatalog(), {
    commands: [],
  });

  await assert.rejects(
    createPrismClient(fakeHttp(() => ({ status: 404, json: { error: 'not-found' } }))).getCatalog(),
    (err: unknown) => err instanceof PrismApiError && err.code === 'not-found',
  );
  await assert.rejects(
    createPrismClient(fakeHttp(() => ({ status: 200, json: { nope: 1 } }))).getCatalog(),
    (err: unknown) => err instanceof PrismApiError && err.code === 'malformed-response',
  );
});
