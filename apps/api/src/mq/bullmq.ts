import os from 'node:os';
import * as Sentry from '@sentry/node';
import { logger } from '@typie/lib';
import { Queue, Worker } from 'bullmq';
import Redis from 'ioredis';
import { dev, env, stack } from '@/env';
import { crons, jobs } from './tasks';

const lane = dev ? os.hostname() : stack;
const taskMap = Object.fromEntries([...jobs, ...crons].map((job) => [job.name, job.fn]));

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

const worker = new Worker(
  lane,
  async (job) => {
    const fn = taskMap[job.name];
    await fn?.(job.data);
  },
  {
    prefix: `${stack}:{mq}`,
    connection: new Redis.Cluster([env.REDIS_URL]),
    concurrency: 50,
  },
);

worker.on('completed', (job) => {
  logger.info`Job ${job.id} (${job.name}) completed`;
});

worker.on('failed', (job, error) => {
  logger.error`Job ${job?.id} (${job?.name}) failed: ${error}`;
  Sentry.captureException(error);
});

worker.on('error', (error) => {
  logger.error`Job error: ${error}`;
  Sentry.captureException(error);
});
