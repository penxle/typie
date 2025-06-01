import '@/instrument';
import '@typie/lib/dayjs';
import '@/mq';

import { serve } from '@hono/node-server';
import { logger } from '@typie/lib';
import { compress } from 'hono/compress';
import { HTTPException } from 'hono/http-exception';
import { app } from '@/app';
import { deriveContext } from '@/context';
import { env } from '@/env';
import { graphql } from '@/graphql';
import { rest } from '@/rest';
import { injectWebSocket } from '@/ws';

app.use('*', compress());

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

app.onError((err, c) => {
  if (err instanceof HTTPException) {
    return err.getResponse();
  }

  logger.error(err);

  return c.text('Internal Server Error', { status: 500 });
});

const server = serve(
  {
    fetch: app.fetch,
    hostname: '0.0.0.0',
    port: env.LISTEN_PORT ?? 3000,
  },
  (addr) => {
    console.log(`Listening on ${addr.address}:${addr.port}`);
  },
);

injectWebSocket(server);
