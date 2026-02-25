import { createRedisEventTarget } from '@graphql-yoga/redis-event-target';
import { createPubSub } from 'graphql-yoga';
import { Redis } from 'ioredis';
import { env } from '@/env';
import type { DocumentSyncType, PostSyncType } from '@/enums';

export const pubsub = createPubSub<{
  'post:sync': [postId: string, { target: string; type: PostSyncType; data: string }];
  'document:sync': [documentId: string, { target: string; type: DocumentSyncType; data: string }];
  'site:update': [siteId: string, { scope: 'site' } | { scope: 'entity'; entityId: string }];
  'site:usage:update': [siteId: string, null];
  'user:usage:update': [userId: string, null];
}>({
  eventTarget: createRedisEventTarget({
    publishClient: new Redis({ name: 'primary', sentinels: [{ host: env.REDIS_URL }] }),
    subscribeClient: new Redis({ name: 'primary', sentinels: [{ host: env.REDIS_URL }] }),
  }),
});
