import '@/instrument';
import '@typie/lib/dayjs';
import '@/mq';

import { serve } from '@hono/node-server';
import { logger } from '@typie/lib';
import { HTTPException } from 'hono/http-exception';
import { app } from '@/app';
import { deriveContext } from '@/context';
import { env } from '@/env';
import { graphql } from '@/graphql';
import { rest } from '@/rest';
import { injectWebSocket } from '@/ws';

const log = logger.getChild('main');

app.use('*', async (c, next) => {
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

const server = serve(
  {
    fetch: app.fetch,
    hostname: '0.0.0.0',
    port: env.LISTEN_PORT ?? 3000,
  },
  (addr) => {
    log.info('Listening {*}', { address: addr.address, port: addr.port });
  },
);

injectWebSocket(server);
