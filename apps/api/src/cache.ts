import { Redis } from 'ioredis';
import { env } from '#/env.ts';

export const redis = new Redis({
  name: 'primary',
  sentinels: [{ host: env.REDIS_URL }],
});
