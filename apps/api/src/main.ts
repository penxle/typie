import '@glitter/lib/dayjs';

import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { env } from '@/env';
import { hono } from '@/rest';

const app = new Hono();

app.use(
  '*',
  cors({
    origin: (origin) => origin,
    credentials: true,
  }),
);

app.route('/', hono);

const server = Bun.serve({
  fetch: app.fetch,
  port: env.LISTEN_PORT ?? 3000,
  idleTimeout: 0,
});

console.log(`Listening on ${server.url}`);
