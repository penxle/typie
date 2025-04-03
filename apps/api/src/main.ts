import '@/instrument';
import '@typie/lib/dayjs';
import '@/mq';

import { and, eq } from 'drizzle-orm';
import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { deriveContext } from '@/context';
import { db, first, Sites } from '@/db';
import { SiteState } from '@/enums';
import { env } from '@/env';
import { yoga } from '@/graphql';
import { rest } from '@/rest';
import type { Env } from '@/context';

const app = new Hono<Env>();

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

  if (url.origin === env.WEBSITE_URL || url.hostname === env.USERSITE_HOST) {
    return handler(c, next);
  }

  const pattern = new RegExp(`^([^.]+)\\.${env.USERSITE_HOST}$`);
  const slug = url.hostname.match(pattern)?.[1];
  if (slug) {
    const site = await db
      .select({ id: Sites.id })
      .from(Sites)
      .where(and(eq(Sites.slug, slug), eq(Sites.state, SiteState.ACTIVE)))
      .then(first);

    if (site) {
      return handler(c, next);
    }
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

const server = Bun.serve({
  fetch: app.fetch,
  port: env.LISTEN_PORT ?? 3000,
  idleTimeout: 0,
});

console.log(`Listening on ${server.url}`);
