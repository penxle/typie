import { Hono } from 'hono';
import { healthz } from './healthz';
import { og } from './og';
import { payment } from './payment';
import { ws } from './ws';
import type { Env } from '@/context';

export const rest = new Hono<Env>();

rest.route('/healthz', healthz);
rest.route('/og', og);
rest.route('/payment', payment);
rest.route('/ws', ws);
