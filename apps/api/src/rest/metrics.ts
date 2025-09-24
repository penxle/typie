import { Hono } from 'hono';
import { queue } from '@/mq/bullmq';
import type { Env } from '@/context';

export const metrics = new Hono<Env>();

metrics.get('/bullmq', async (c) => {
  const metrics = await queue.exportPrometheusMetrics();
  return c.text(metrics);
});
