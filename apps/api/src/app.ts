import { Hono } from 'hono';
import type { Env } from '@/context';

export const app = new Hono<Env>();
