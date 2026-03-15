import { Hono } from 'hono';
import type { Env } from '#/context.ts';

export const app = new Hono<Env>();
