import os from 'node:os';
import nc from 'node-cron';
import { dev, stack } from '@/env';
import { rabbit } from './connection';
import { crons } from './tasks';
import type { JobMap, JobName } from './tasks';
import type { JobFn } from './types';

const routingKey = dev ? os.hostname() : stack;

export const publisher = rabbit.createPublisher({
  confirm: true,
  maxAttempts: 3,

  exchanges: [
    { exchange: 'tasks', type: 'topic', durable: true },
    { exchange: 'delayed-tasks', type: 'topic', durable: true },
  ],

  queues: [
    { queue: 'tasks', durable: true, arguments: { 'x-queue-type': 'quorum' } },
    { queue: 'delayed-tasks', durable: true, arguments: { 'x-queue-type': 'quorum', 'x-dead-letter-exchange': 'tasks' } },
  ],

  queueBindings: [
    { exchange: 'tasks', queue: 'tasks', routingKey },
    { exchange: 'delayed-tasks', queue: 'delayed-tasks', routingKey },
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
      exchange: options?.delay ? 'delayed-tasks' : 'tasks',
      routingKey,
      durable: true,
      priority: options?.priority ?? 5,
      ...(options?.delay && { expiration: options.delay.toString() }),
    },
    { name, data },
  );
};

for (const cron of crons) {
  nc.schedule(
    cron.pattern,
    () => {
      enqueueJob(cron.name as JobName, null as never);
    },
    { name: cron.name, timezone: 'Asia/Seoul' },
  );
}
