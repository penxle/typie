import os from 'node:os';
import { createPubSub } from 'graphql-yoga';
import { nanoid } from 'nanoid';
import { dev } from '@/env';
import { publisher, rabbit } from '@/mq';
import type { CanvasSyncType, PostSyncType } from '@/enums';

const queue = dev ? `pubsub:local:${os.hostname()}:${nanoid()}` : `pubsub:${nanoid()}`;
const routingKey = dev ? os.hostname() : 'default';
const subscriptions = new Map<string, Set<EventListener>>();

rabbit.createConsumer(
  {
    queue,
    queueOptions: { queue, exclusive: true },
    queueBindings: [{ exchange: 'pubsub', queue, routingKey }],
    exchanges: [{ exchange: 'pubsub', type: 'topic', durable: true }],

    qos: { prefetchCount: 20 },
  },
  (msg) => {
    const { type, detail } = msg.body;
    const listeners = subscriptions.get(type);
    listeners?.forEach((listener) => listener(new CustomEvent(type, { detail })));
  },
);

export const pubsub = createPubSub<{
  'post:sync': [postId: string, { target: string; type: PostSyncType; data: string }];
  'canvas:sync': [canvasId: string, { target: string; type: CanvasSyncType; data: string }];
  'site:update': [siteId: string, { scope: 'site' } | { scope: 'entity'; entityId: string }];
  'site:usage:update': [siteId: string, null];
}>({
  eventTarget: {
    dispatchEvent: (event) => {
      publisher.send({ exchange: 'pubsub', routingKey }, { type: event.type, detail: event.detail });
      return true;
    },
    addEventListener: (type, listener) => {
      const listeners = subscriptions.get(type) ?? new Set();
      listeners.add(listener as EventListener);
      subscriptions.set(type, listeners);
    },
    removeEventListener: (type, listener) => {
      const listeners = subscriptions.get(type);
      listeners?.delete(listener as EventListener);
    },
  },
});
