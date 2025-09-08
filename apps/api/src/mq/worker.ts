import os from 'node:os';
import * as Sentry from '@sentry/bun';
import { logger } from '@typie/lib';
import { dev } from '@/env';
import { rabbit } from './connection';
import { crons, jobs } from './tasks';

const log = logger.getChild('mq');
const taskMap = Object.fromEntries([...jobs, ...crons].map((job) => [job.name, job.fn]));

const queue = dev ? `tasks:local:${os.hostname()}` : 'tasks';

const consumer = rabbit.createConsumer(
  {
    queue,
    queueOptions: { queue, durable: true, arguments: { 'x-queue-type': 'quorum' } },
    queueBindings: [{ exchange: 'tasks', queue, routingKey: queue }],
    exchanges: [{ exchange: 'tasks', type: 'direct', durable: true }],

    qos: { prefetchCount: 20 },
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
