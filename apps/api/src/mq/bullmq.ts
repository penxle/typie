import os from 'node:os';
import * as Sentry from '@sentry/node';
import { logger } from '@typie/lib';
import { Queue, Worker } from 'bullmq';
import { Redis } from 'ioredis';
import { dev, env, stack } from '#/env.ts';
import { crons, jobs } from './tasks/index.ts';

const log = logger.getChild('mq');
const lane = dev ? os.hostname() : stack;
const taskMap = Object.fromEntries([...jobs, ...crons].map((job) => [job.name, job.fn]));
const cronNames = new Set(crons.map((c) => c.name));

export const queue = new Queue(lane, {
  connection: new Redis({
    host: env.REDIS_URL,
    tls: {},
    maxRetriesPerRequest: null,
  }),

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

export const worker = new Worker(
  lane,
  async (job) => {
    const fn = taskMap[job.name];
    if (cronNames.has(job.name)) {
      await Sentry.withMonitor(job.name, () => fn?.(job.data));
    } else {
      await fn?.(job.data);
    }
  },
  {
    connection: new Redis({
      host: env.REDIS_URL,
      tls: {},
      maxRetriesPerRequest: null,
    }),

    autorun: false,
    concurrency: 10,
    lockDuration: 120_000,
  },
);

worker.on('completed', (job) => {
  log.info('Job completed {*}', { id: job.id, name: job.name, data: job.data });
});

worker.on('failed', (job, error) => {
  log.error('Job failed {*}', { id: job?.id, name: job?.name, data: job?.data, error });
  Sentry.captureException(error, {
    extra: { jobId: job?.id, jobName: job?.name, jobData: job?.data },
  });
});

worker.on('error', (error) => {
  log.error('Job error {*}', { error });
  Sentry.captureException(error);
});

if (!process.env.SCRIPT && !process.env.NO_WORKER) {
  worker.run();
}
