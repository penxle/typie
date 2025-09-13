import { Hono } from 'hono';
import { auth } from './auth';
import { bmo } from './bmo';
import { gir } from './gir';
import { healthz } from './healthz';
import { iap } from './iap';
import { og } from './og';
import type { Env } from '@/context';

export const rest = new Hono<Env>();

rest.route('/auth', auth);
rest.route('/bmo', bmo);
rest.route('/gir', gir);
rest.route('/healthz', healthz);
rest.route('/iap', iap);
rest.route('/og', og);
