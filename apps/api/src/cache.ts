import { Redis } from 'ioredis';
import { env } from '#/env.ts';

export const redis = new Redis({ host: env.REDIS_URL, tls: {} });
