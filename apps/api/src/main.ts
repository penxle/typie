import '@/instrument';
import '@typie/lib/dayjs';
import '@/mq';

import escape from 'escape-string-regexp';
import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { compress } from 'hono-compress';
import { deriveContext } from '@/context';
import { env } from '@/env';
import { yoga } from '@/graphql';
import { rest } from '@/rest';
import { websocket } from '@/ws';
import type { Env } from '@/context';

const app = new Hono<Env>();
const pattern = new RegExp(`^${escape(env.USERSITE_URL).replace(String.raw`\*\.`, String.raw`(([^.]+)\.)?`)}$`);

app.use('*', compress());

app.use('*', async (c, next) => {
  const origin = c.req.header('Origin');
  if (!origin) {
    return next();
  }

  const url = new URL(origin);
  const handler = cors({
    origin,
    credentials: true,
  });

  if (url.origin === env.WEBSITE_URL || pattern.test(url.origin)) {
    return handler(c, next);
  }

  return next();
});

app.use('*', async (c, next) => {
  const context = await deriveContext(c);
  c.set('context', context);

  return next();
});

app.route('/', rest);
app.route('/graphql', yoga);

app.onError((_, c) => {
  return c.text('Internal Server Error', { status: 500 });
});

const server = Bun.serve({
  fetch: app.fetch,
  error: (err) => {
    if (err.code === 'ENOENT') {
      return new Response('Not Found', {
        status: 404,
      });
    }

    return new Response('Internal Server Error', {
      status: 500,
    });
  },
  websocket,
  port: env.LISTEN_PORT ?? 3000,
  idleTimeout: 0,
});

console.log(`Listening on ${server.url}`);
