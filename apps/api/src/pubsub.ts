import { createRedisEventTarget } from '@graphql-yoga/redis-event-target';
import { createPubSub } from 'graphql-yoga';
import { Redis } from 'ioredis';
import { env } from '@/env';

export const pubsub = createPubSub<{
  'site:update': [siteId: string, { scope: 'site' } | { scope: 'entity'; entityId: string }];
}>({
  eventTarget: createRedisEventTarget({
    publishClient: new Redis(env.REDIS_URL),
    subscribeClient: new Redis(env.REDIS_URL),
  }),
});
