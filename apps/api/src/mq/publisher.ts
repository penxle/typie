import os from 'node:os';
import { dev } from '@/env';
import { rabbit } from './connection';
import type { JobMap, JobName } from './tasks';
import type { JobFn } from './types';

const queue = dev ? `tasks:local:${os.hostname()}` : 'tasks';
const delayedQueue = dev ? `tasks:delayed:local:${os.hostname()}` : 'tasks:delayed';

export const publisher = rabbit.createPublisher({
  confirm: true,
  maxAttempts: 3,

  exchanges: [
    { exchange: 'tasks', type: 'direct', durable: true },
    { exchange: 'tasks:delayed', type: 'direct', durable: true },
    { exchange: 'pubsub', type: 'topic', durable: true },
  ],

  queues: [
    { queue, durable: true, arguments: { 'x-queue-type': 'quorum' } },
    {
      queue: delayedQueue,
      durable: true,
      arguments: { 'x-queue-type': 'quorum', 'x-dead-letter-exchange': 'tasks', 'x-dead-letter-routing-key': queue },
    },
  ],

  queueBindings: [
    { exchange: 'tasks', queue, routingKey: queue },
    { exchange: 'tasks:delayed', queue: delayedQueue, routingKey: delayedQueue },
  ],
});

type EnqueueJobOptions = {
  delay?: number;
  priority?: number;
};

export const enqueueJob = async <N extends JobName, F extends JobMap[N]>(
  name: N,
  data: F extends JobFn<infer P> ? P : never,
  options?: EnqueueJobOptions,
) => {
  await publisher.send(
    {
      exchange: options?.delay ? 'tasks:delayed' : 'tasks',
      routingKey: options?.delay ? delayedQueue : queue,
      durable: true,
      priority: options?.priority ?? 5,
      ...(options?.delay && { expiration: options.delay.toString() }),
    },
    { name, data },
  );
};
