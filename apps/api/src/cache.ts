import { Redlock } from '@sesamecare-oss/redlock';
import { Redis } from 'ioredis';
import { env, stack } from '@/env';

export const redis = new Redis.Cluster([env.REDIS_URL], {
  keyPrefix: `${stack}:`,
});

export const redlock = new Redlock([redis]);
