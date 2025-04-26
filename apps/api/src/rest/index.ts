import { Hono } from 'hono';
import { auth } from './auth';
import { healthz } from './healthz';
import { og } from './og';
import { payment } from './payment';
import type { Env } from '@/context';

export const rest = new Hono<Env>();

rest.route('/auth', auth);
rest.route('/healthz', healthz);
rest.route('/og', og);
rest.route('/payment', payment);
