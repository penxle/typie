import '@/instrument';
import '@typie/lib/dayjs';
import '@/mq';

import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { env } from '@/env';
import { yoga } from '@/graphql';
import { rest } from '@/rest';

const app = new Hono();

app.use('*', async (c, next) => {
  const origin = c.req.header('origin');

  if (origin === env.WEBSITE_URL) {
    return cors({
      origin,
      credentials: true,
    })(c, next);
  }

  return next();
});

app.route('/', rest);
app.route('/graphql', yoga);

const server = Bun.serve({
  fetch: app.fetch,
  port: env.LISTEN_PORT ?? 3000,
  idleTimeout: 0,
});

console.log(`Listening on ${server.url}`);
