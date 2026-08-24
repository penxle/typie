import { createRedisEventTarget } from '@graphql-yoga/redis-event-target';
import { createPubSub } from 'graphql-yoga';
import { Redis } from 'ioredis';
import { env } from '#/env.ts';
import type { ProjectedStreamFrame } from '@typie/prism';

export const NOTE_UPDATE_KINDS = ['CREATED', 'UPDATED', 'DELETED'] as const;

export const pubsub = createPubSub<{
  'document:changesets': [
    documentId: string,
    payload:
      | { target: string; seq: string; changesets: string[]; heads: string; durableHeads: string }
      | { kind: 'head-isolated'; userId: string; headId: string; excluded: boolean },
  ];
  'document:comment': [documentId: string, payload: { threadId: string; originClientId: string }];
  'note:update': [siteId: string, payload: { kind: (typeof NOTE_UPDATE_KINDS)[number]; noteId: string; originClientId?: string }];
  'prism:review': [documentId: string, payload: { roundId: string }];
  'prism:session': [sessionId: string, payload: ProjectedStreamFrame];
  'site:update': [siteId: string, payload: { scope: 'site' } | { scope: 'entity'; entityId: string }];
  'site:usage:update': [siteId: string, payload: null];
  'user:usage:update': [userId: string, payload: null];
}>({
  eventTarget: createRedisEventTarget({
    publishClient: new Redis({ host: env.REDIS_URL, tls: {} }),
    subscribeClient: new Redis({ host: env.REDIS_URL, tls: {} }),
  }),
});
