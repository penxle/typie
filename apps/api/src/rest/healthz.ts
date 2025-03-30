import { Hono } from 'hono';
import type { Env } from '@/context';

export const healthz = new Hono<Env>();

healthz.get('/', (c) => {
  return c.json({ '*': true });
});
