import { Hono } from 'hono';
import { healthz } from './healthz';
import { payment } from './payment';

export const rest = new Hono();

rest.route('/healthz', healthz);
rest.route('/payment', payment);
