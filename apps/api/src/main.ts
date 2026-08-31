import '@typie/lib/dayjs';
import '#/mq/index.ts';

import { serve } from '@hono/node-server';
import * as Sentry from '@sentry/node';
import { getClientAddress, logger, withContext } from '@typie/lib';
import { HTTPException } from 'hono/http-exception';
import { WebSocketServer } from 'ws';
import { app } from '#/app.ts';
import { checkBootstrap } from '#/bootstrap.ts';
import { deriveContext } from '#/context.ts';
import { env } from '#/env.ts';
import { graphql } from '#/graphql/index.ts';
import { stopPrismWorker } from '#/mq/prism-worker.ts';
import { rest } from '#/rest/index.ts';
import { attachSyncServer } from '#/sync/index.ts';

const log = logger.getChild('main');

app.use('*', async (c, next) => {
  if (c.req.path.startsWith('/healthz') || c.req.path.startsWith('/bmo/') || c.req.path.startsWith('/internal/')) {
    return next();
  }

  const { maintenance } = await checkBootstrap(getClientAddress(c), c.req.header('X-Bootstrap-Bypass'));
  if (maintenance) {
    return c.json({ code: 'under_maintenance' as const, ...maintenance }, 503);
  }

  const context = await deriveContext(c);
  c.set('context', context);

  return Sentry.withIsolationScope((scope) => {
    if (context.session) {
      scope.setUser({ id: context.session.userId });
    }

    scope.setTransactionName(`${c.req.method} ${c.req.path}`);
    scope.setContext('request', {
      method: c.req.method,
      url: c.req.url,
      query: c.req.query(),
    });

    return withContext({ userId: context.session?.userId, ip: context.ip }, next);
  });
});

app.route('/', rest);
app.route('/graphql', graphql);

app.notFound((c) => {
  return c.text('Not Found', { status: 404 });
});

app.onError((error, c) => {
  if (error instanceof HTTPException) {
    return error.getResponse();
  }

  log.error('Unhandled error {*}', { error });
  Sentry.captureException(error);

  return c.text('Internal Server Error', { status: 500 });
});

const wss = new WebSocketServer({ noServer: true });

const server = serve(
  {
    fetch: app.fetch,
    hostname: '0.0.0.0',
    port: env.LISTEN_PORT ?? 3000,
    websocket: { server: wss },
  },
  (info) => {
    log.info('Listening {*}', { hostname: info.address, port: info.port });
  },
);

const httpServer = server as import('node:http').Server;
const sync = attachSyncServer(httpServer);

const SHUTDOWN_TIMEOUT_MS = 20_000;

const shutdown = async () => {
  for (const ws of wss.clients) {
    ws.close(1001, 'Server is shutting down');
  }

  sync.close();

  const closed = new Promise<void>((resolve) => {
    httpServer.close(() => resolve());
  });

  httpServer.closeIdleConnections();

  await Promise.all([closed, stopPrismWorker()]);
};

process.once('SIGTERM', () => {
  log.info('Received SIGTERM, shutting down');

  const timeout = setTimeout(() => {
    log.warn('Shutdown timed out, exiting');
    process.exit(0);
  }, SHUTDOWN_TIMEOUT_MS);
  timeout.unref();

  void shutdown()
    .catch((err) => {
      log.error('Shutdown failed {*}', { error: err });
    })
    .finally(() => {
      clearTimeout(timeout);
      process.exit(0);
    });
});
