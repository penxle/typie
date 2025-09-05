import { Redis } from 'ioredis';
import { env } from '@/env';

export const redis = new Redis({
  name: 'primary',
  sentinels: [{ host: env.REDIS_URL }],
});
