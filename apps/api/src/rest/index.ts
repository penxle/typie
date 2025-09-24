import { Hono } from 'hono';
import { auth } from './auth';
import { bmo } from './bmo';
import { healthz } from './healthz';
import { iap } from './iap';
import { metrics } from './metrics';
import { og } from './og';
import type { Env } from '@/context';

export const rest = new Hono<Env>();

rest.route('/auth', auth);
rest.route('/bmo', bmo);
rest.route('/healthz', healthz);
rest.route('/iap', iap);
rest.route('/metrics', metrics);
rest.route('/og', og);
