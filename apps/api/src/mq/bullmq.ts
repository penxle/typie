import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { isMainThread } from 'node:worker_threads';
import * as Sentry from '@sentry/node';
import { logger } from '@typie/lib';
import { Queue, Worker } from 'bullmq';
import Redis from 'ioredis';
import { dev, env, stack } from '@/env';
import { crons } from './tasks';

const __dirname = fileURLToPath(new URL('.', import.meta.url));

const log = logger.getChild('mq');
const lane = dev ? os.hostname() : stack;

export const queue = new Queue(lane, {
  prefix: `${stack}:{mq}`,
  connection: new Redis.Cluster([env.REDIS_URL]),

  defaultJobOptions: {
    removeOnComplete: true,
    removeOnFail: true,

    attempts: 3,
    backoff: {
      type: 'exponential',
      delay: 1000,
    },
  },
});

if (isMainThread) {
  for (const cron of crons) {
    queue.upsertJobScheduler(cron.name, {
      pattern: cron.pattern,
      tz: 'Asia/Seoul',
    });
  }

  if (!process.env.SCRIPT && !process.env.NO_WORKER) {
    const count = dev ? 1 : 4;

    log.info('Starting workers {*}', { count });

    for (let i = 0; i < count; i++) {
      const worker = new Worker(lane, path.join(__dirname, 'worker.ts'), {
        prefix: `${stack}:{mq}`,
        connection: new Redis.Cluster([env.REDIS_URL]),
        concurrency: 100,
        useWorkerThreads: true,
      });

      worker.on('completed', (job) => {
        log.info('Job completed {*}', { id: job.id, name: job.name });
      });

      worker.on('failed', (job, error) => {
        log.error('Job failed {*}', { id: job?.id, name: job?.name, error });
        Sentry.captureException(error);
      });

      worker.on('error', (error) => {
        log.error('Job error {*}', { error });
        Sentry.captureException(error);
      });
    }
  }
}
