import { Redis } from 'ioredis';
import { env, production } from '#/env.ts';
import type { RedisOptions } from 'ioredis';

const options: RedisOptions = production ? { name: 'primary', sentinels: [{ host: env.REDIS_URL }] } : { host: env.REDIS_URL, tls: {} };
export const redis = new Redis(options);
