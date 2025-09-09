import { createRedisEventTarget } from '@graphql-yoga/redis-event-target';
import { createPubSub } from 'graphql-yoga';
import { Redis } from 'ioredis';
import { env } from '@/env';
import type { CanvasSyncType, PostSyncType } from '@/enums';

export const pubsub = createPubSub<{
  'post:sync': [postId: string, { target: string; type: PostSyncType; data: string }];
  'canvas:sync': [canvasId: string, { target: string; type: CanvasSyncType; data: string }];
  'site:update': [siteId: string, { scope: 'site' } | { scope: 'entity'; entityId: string }];
  'site:usage:update': [siteId: string, null];
}>({
  eventTarget: createRedisEventTarget({
    publishClient: new Redis({ name: 'primary', sentinels: [{ host: env.REDIS_URL }] }),
    subscribeClient: new Redis({ name: 'primary', sentinels: [{ host: env.REDIS_URL }] }),
  }),
});
