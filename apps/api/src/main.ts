import '@typie/lib/dayjs';
import '@/mq';

import { getClientAddress, logger } from '@typie/lib';
import { websocket } from 'hono/bun';
import { HTTPException } from 'hono/http-exception';
import { app } from '@/app';
import { checkBootstrap } from '@/bootstrap';
import { deriveContext } from '@/context';
import { env } from '@/env';
import { graphql } from '@/graphql';
import { rest } from '@/rest';

const log = logger.getChild('main');

app.use('*', async (c, next) => {
  if (c.req.path.startsWith('/healthz') || c.req.path.startsWith('/bmo/')) {
    return next();
  }

  const { maintenance } = await checkBootstrap(getClientAddress(c), c.req.header('X-Bootstrap-Bypass'));
  if (maintenance) {
    return c.json({ code: 'under_maintenance' as const, ...maintenance }, 503);
  }

  const context = await deriveContext(c);
  c.set('context', context);

  return next();
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

  return c.text('Internal Server Error', { status: 500 });
});

const server = Bun.serve({
  fetch: app.fetch,
  hostname: '0.0.0.0',
  port: env.LISTEN_PORT ?? 3000,
  idleTimeout: 60,
  websocket: {
    ...websocket,
    idleTimeout: 60,
    perMessageDeflate: true,
    maxPayloadLength: 16 * 1024 * 1024, // 16 MB
    backpressureLimit: 16 * 1024 * 1024, // 16 MB
  },
});

log.info('Listening {*}', { hostname: server.hostname, port: server.port });
