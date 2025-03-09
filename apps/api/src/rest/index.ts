import { Hono } from 'hono';
import { healthz } from './healthz';

export const rest = new Hono();

rest.route('/healthz', healthz);
