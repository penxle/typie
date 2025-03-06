import { Hono } from 'hono';
import { healthz } from './healthz';

export const hono = new Hono();

hono.route('/healthz', healthz);
