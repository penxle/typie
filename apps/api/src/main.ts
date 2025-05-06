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

const server = serve({
  fetch: app.fetch,
  port: env.LISTEN_PORT ?? 3000,
});

injectWebSocket(server);

console.log(`Listening on http://localhost:${env.LISTEN_PORT ?? 3000}`);
