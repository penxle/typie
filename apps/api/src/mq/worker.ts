import os from 'node:os';
import * as Sentry from '@sentry/bun';
import { logger } from '@typie/lib';
import { dev, stack } from '@/env';
import { rabbit } from './connection';
import { crons, jobs } from './tasks';

const log = logger.getChild('mq');
const routingKey = dev ? os.hostname() : stack;
const taskMap = Object.fromEntries([...jobs, ...crons].map((job) => [job.name, job.fn]));

const consumer = rabbit.createConsumer(
  {
    queue: 'tasks',
    queueOptions: { queue: 'tasks', durable: true, arguments: { 'x-queue-type': 'quorum' } },
    queueBindings: [{ exchange: 'tasks', queue: 'tasks', routingKey }],
    exchanges: [{ exchange: 'tasks', type: 'topic', durable: true }],

    qos: { prefetchCount: 2 },
    lazy: true,
  },
  async (msg) => {
    try {
      const { name, data } = msg.body;

      const fn = taskMap[name];
      await fn?.(data);

      log.info('Job completed {*}', { name });
    } catch (err) {
      log.error('Job error {*}', { error: err });
      Sentry.captureException(err);
      throw err;
    }
  },
);

consumer.on('error', (error) => {
  log.error('Worker error {*}', { error });
  Sentry.captureException(error);
});

if (!process.env.NO_WORKER) {
  consumer.start();
}
