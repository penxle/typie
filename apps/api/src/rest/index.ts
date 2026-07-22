import { Hono } from 'hono';
import { auth } from './auth.ts';
import { bmo } from './bmo.ts';
import { entity } from './entity.ts';
import { font } from './font.ts';
import { healthz } from './healthz.ts';
import { iap } from './iap.ts';
import { internal } from './internal.ts';
import { og } from './og.tsx.js';
import type { Env } from '#/context.ts';

export const rest = new Hono<Env>();

rest.route('/auth', auth);
rest.route('/bmo', bmo);
rest.route('/entity', entity);
rest.route('/font', font);
rest.route('/healthz', healthz);
rest.route('/iap', iap);
rest.route('/internal', internal);
rest.route('/og', og);
