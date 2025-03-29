import os from 'node:os';
import * as Sentry from '@sentry/bun';
import { logger } from '@typie/lib';
import { Queue, Worker } from 'bullmq';
import { dev, env } from '@/env';
import { crons, jobs } from './tasks';

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const lane = dev ? os.hostname() : env.PUBLIC_PULUMI_STACK!;
const taskMap = Object.fromEntries([...jobs, ...crons].map((job) => [job.name, job.fn]));

export const queue = new Queue(lane, {
  connection: { url: env.REDIS_URL },
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
    await fn(job.data);
  },
  {
    connection: { url: env.REDIS_URL },
  },
);

worker.on('completed', (job) => {
  logger.info(`Job ${job.id} (${job.name}) completed`);
});

worker.on('failed', (job, error) => {
  logger.error(error, `Job ${job?.id} (${job?.name}) failed`);
  Sentry.captureException(error);
});

worker.on('error', (error) => {
  logger.error(error);
  Sentry.captureException(error);
});
