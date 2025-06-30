import { Hono } from 'hono';
import { auth } from './auth';
import { clair } from './clair';
import { healthz } from './healthz';
import { iap } from './iap';
import { og } from './og';
import { payment } from './payment';
import type { Env } from '@/context';

export const rest = new Hono<Env>();

rest.route('/auth', auth);
rest.route('/clair', clair);
rest.route('/healthz', healthz);
rest.route('/iap', iap);
rest.route('/og', og);
rest.route('/payment', payment);
