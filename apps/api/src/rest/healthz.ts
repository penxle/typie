import { sql } from 'drizzle-orm';
import { Hono } from 'hono';
import { redis } from '@/cache';
import { db } from '@/db';
import type { Env } from '@/context';

export const healthz = new Hono<Env>();

healthz.get('/', (c) => {
  return c.json({ '*': true });
});

healthz.get('/liveness', (c) => {
  return c.json({ '*': true });
});

healthz.get('/readiness', async (c) => {
  const checks = {
    database: false,
    redis: false,
  };

  try {
    await db.execute(sql`SELECT 1`);
    checks.database = true;
  } catch {
    // pass
  }

  try {
    await redis.ping();
    checks.redis = true;
  } catch {
    // pass
  }

  const all = Object.values(checks).every((check) => check === true);
  if (all) {
    return c.json({ '*': true, ...checks });
  } else {
    return c.json({ '*': false, ...checks }, 503);
  }
});
