import { createRedisEventTarget } from '@graphql-yoga/redis-event-target';
import { createPubSub } from 'graphql-yoga';
import { Redis } from 'ioredis';
import { env, production } from '#/env.ts';
import type { DocumentSyncType } from '@typie/lib/enums';
import type { RedisOptions } from 'ioredis';

const options: RedisOptions = production ? { name: 'primary', sentinels: [{ host: env.REDIS_URL }] } : { host: env.REDIS_URL, tls: {} };

export const pubsub = createPubSub<{
  'document:sync': [documentId: string, { target: string; type: DocumentSyncType; data: string }];
  'document:commits': [documentId: string, { commitIds: string[]; objectIds: string[] }];
  'site:update': [siteId: string, { scope: 'site' } | { scope: 'entity'; entityId: string }];
  'site:usage:update': [siteId: string, null];
  'user:usage:update': [userId: string, null];
}>({
  eventTarget: createRedisEventTarget({
    publishClient: new Redis(options),
    subscribeClient: new Redis(options),
  }),
});
