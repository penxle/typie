import * as Sentry from '@sentry/node';
import { logger } from '@typie/lib';
import { Worker } from 'bullmq';
import { PRISM_LANE, prismRedis, shutdown } from './prism-queue.ts';
import { processIngestJob } from './tasks/prism-ingest.ts';

const log = logger.getChild('prism-mq');

export const prismWorker = new Worker<{ logKey: string }>(PRISM_LANE, (job, token, signal) => processIngestJob(job, token, signal), {
  connection: prismRedis(),

  autorun: false,
  concurrency: Number.MAX_SAFE_INTEGER,
  lockDuration: 30_000,
  stalledInterval: 15_000,
  maxStalledCount: 100,
});

prismWorker.on('failed', (job, error) => {
  log.error('Ingest job failed {*}', { id: job?.id, error });
  Sentry.captureException(error, { extra: { jobId: job?.id } });
});

prismWorker.on('error', (error) => {
  log.error('Ingest worker error {*}', { error });
  Sentry.captureException(error);
});

if (!process.env.SCRIPT && !process.env.NO_WORKER) {
  prismWorker.run();

  process.once('SIGTERM', () => {
    shutdown.abort();
    void Promise.race([prismWorker.close(), new Promise((resolve) => setTimeout(resolve, 5000))]).finally(() => process.exit(0));
  });
}
