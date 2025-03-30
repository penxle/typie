import { Hono } from 'hono';
import { healthz } from './healthz';
import { payment } from './payment';
import type { Env } from '@/context';

export const rest = new Hono<Env>();

rest.route('/healthz', healthz);
rest.route('/payment', payment);
