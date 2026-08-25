import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import test from 'node:test';
import { Agent as Http2Agent } from 'http2-wrapper';
import { createPrismHttp } from './prism-http.ts';
import type { IncomingMessage, ServerResponse } from 'node:http';
import type { AddressInfo, Socket } from 'node:net';

const startServer = async (handler: (req: IncomingMessage, res: ServerResponse) => void) => {
  const sockets = new Set<Socket>();
  const server = createServer(handler);
  server.on('connection', (socket) => {
    sockets.add(socket);
    socket.on('close', () => sockets.delete(socket));
  });
  await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', () => resolve()));
  const { port } = server.address() as AddressInfo;

  return {
    baseUrl: `http://127.0.0.1:${port}`,
    close: async () => {
      for (const socket of sockets) socket.destroy();
      await new Promise<void>((resolve) => server.close(() => resolve()));
    },
  };
};

const readBody = async (req: IncomingMessage) => {
  const chunks: Buffer[] = [];
  for await (const chunk of req) chunks.push(chunk as Buffer);
  return Buffer.concat(chunks).toString();
};

test('응답이 유실된 POST는 같은 body(key)로 재시도되고 두 번째 응답을 돌려준다', async () => {
  const bodies: string[] = [];
  const attempts: number[] = [];
  const server = await startServer((req, res) => {
    void readBody(req).then((body) => {
      bodies.push(body);
      if (bodies.length === 1) return;
      res.writeHead(200, { 'content-type': 'application/json' });
      res.end(JSON.stringify({ runSeq: 7 }));
    });
  });

  try {
    const http = createPrismHttp({
      baseUrl: server.baseUrl,
      token: 't',
      timeout: 100,
      totalTimeout: 5000,
      onRetry: ({ attempt }) => void attempts.push(attempt),
    });
    const res = await http.request('/agents/typie-1/resume', { method: 'POST', body: { message: '안녕', key: 'k1' } });

    assert.equal(res.status, 200);
    assert.deepEqual(await res.json(), { runSeq: 7 });
    assert.equal(bodies.length, 2);
    assert.equal(bodies[0], bodies[1]);
    assert.equal(JSON.parse(bodies[1]).key, 'k1');
    assert.deepEqual(attempts, [1]);
  } finally {
    await server.close();
  }
});

test('전체 예산이 소진되면 재시도를 멈추고 타임아웃으로 실패한다', async () => {
  let received = 0;
  const server = await startServer((req) => {
    received += 1;
    void readBody(req);
  });

  try {
    const http = createPrismHttp({ baseUrl: server.baseUrl, token: 't', timeout: 50, totalTimeout: 200 });
    await assert.rejects(http.request('/agents', { method: 'POST', body: { key: 'k1' } }), { name: 'TimeoutError' });
    assert.ok(received >= 1 && received <= 3, `received=${received}`);
  } finally {
    await server.close();
  }
});

test('유니어리 재시도는 h2 세션을 파기해 다음 시도가 새 연결을 쓰게 한다', async () => {
  const bodies: string[] = [];
  const server = await startServer((req, res) => {
    void readBody(req).then((body) => {
      bodies.push(body);
      if (bodies.length === 1) return;
      res.writeHead(200, { 'content-type': 'application/json' });
      res.end(JSON.stringify({ runSeq: 1 }));
    });
  });

  try {
    const agent = new Http2Agent();
    let destroyed = 0;
    const originalDestroy = agent.destroy.bind(agent);
    agent.destroy = (reason?: Error) => {
      destroyed += 1;
      originalDestroy(reason);
    };

    const http = createPrismHttp({ baseUrl: server.baseUrl, token: 't', timeout: 100, totalTimeout: 5000, http2Agent: agent });
    const res = await http.request('/agents', { method: 'POST', body: { key: 'k1' } });

    assert.equal(res.status, 200);
    assert.equal(destroyed, 1);
  } finally {
    await server.close();
  }
});

test('열린 스트림이 같은 origin의 후속 요청을 막지 않는다', async () => {
  const server = await startServer((req, res) => {
    if (req.url?.endsWith('/events')) {
      res.writeHead(200, { 'content-type': 'text/event-stream' });
      res.write('event: sync\ndata: {"seq":1}\n\n');
      return;
    }
    void readBody(req).then(() => {
      res.writeHead(200, { 'content-type': 'application/json' });
      res.end(JSON.stringify({ runSeq: 1 }));
    });
  });

  try {
    const http = createPrismHttp({ baseUrl: server.baseUrl, token: 't', timeout: 2000, totalTimeout: 5000 });
    const stream = await http.request('/agents/typie-1/events', { stream: true });
    assert.equal(stream.status, 200);
    assert.ok(stream.body !== null);

    const res = await http.request('/agents/typie-1/resume', { method: 'POST', body: { message: 'a', key: 'k1' } });
    assert.equal(res.status, 200);
    await stream.body.cancel();
  } finally {
    await server.close();
  }
});

test('스트림 열기는 응답 헤더가 오지 않으면 열기 상한 안에 실패한다', async () => {
  const server = await startServer((req) => void readBody(req));

  try {
    const http = createPrismHttp({ baseUrl: server.baseUrl, token: 't', streamOpenTimeout: 120 });
    const startedAt = Date.now();
    await assert.rejects(http.request('/agents/typie-1/events', { stream: true }), /stream open timed out/);
    assert.ok(Date.now() - startedAt < 2000);
  } finally {
    await server.close();
  }
});

test('스트림 열기 타임아웃은 h2 세션을 파기해 다음 시도가 새 연결을 쓰게 한다', async () => {
  const server = await startServer((req) => void readBody(req));

  try {
    const agent = new Http2Agent();
    let destroyed = 0;
    const originalDestroy = agent.destroy.bind(agent);
    agent.destroy = (reason?: Error) => {
      destroyed += 1;
      originalDestroy(reason);
    };

    const http = createPrismHttp({ baseUrl: server.baseUrl, token: 't', streamOpenTimeout: 120, http2Agent: agent });
    await assert.rejects(http.request('/agents/typie-1/events', { stream: true }), /stream open timed out/);
    assert.equal(destroyed, 1);
  } finally {
    await server.close();
  }
});

test('스트림 요청에는 시도 상한도 전체 예산도 걸리지 않는다', async () => {
  const server = await startServer((req, res) => {
    setTimeout(() => {
      res.writeHead(200, { 'content-type': 'text/event-stream' });
      res.end('event: sync\ndata: {"seq":1}\n\n');
    }, 250);
  });

  try {
    const http = createPrismHttp({ baseUrl: server.baseUrl, token: 't', timeout: 50, totalTimeout: 100 });
    const res = await http.request('/agents/typie-1/events', { stream: true, headers: { 'last-event-id': '0' } });

    assert.equal(res.status, 200);
    assert.ok(res.body !== null);
    assert.match(await new Response(res.body).text(), /event: sync/);
  } finally {
    await server.close();
  }
});
