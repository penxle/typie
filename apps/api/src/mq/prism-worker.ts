import * as Sentry from '@sentry/node';
import { logger } from '@typie/lib';
import { Worker } from 'bullmq';
import { LOCK_LOST, PRISM_LANE, prismRedis, shutdown } from './prism-queue.ts';
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

// 락을 잃은 잡은 stalled 판정으로 다른 워커가 이어받는다 — BullMQ는 옛 프로세서를 멈추지 않으므로 여기서 취소하지 않으면
// 같은 대상에 펌프가 둘 돌아 델타가 두 벌 발행되고 도구 실행·푸시·과금도 두 번 시도된다.
prismWorker.on('lockRenewalFailed', (jobIds) => {
  log.warn('Ingest lock lost, cancelling {*}', { jobIds });
  for (const jobId of jobIds) prismWorker.cancelJob(jobId, LOCK_LOST);
});

prismWorker.on('error', (error) => {
  log.error('Ingest worker error {*}', { error });
  Sentry.captureException(error);
});

const running = !process.env.SCRIPT && !process.env.NO_WORKER;

if (running) {
  prismWorker.run();
}

export const stopPrismWorker = async () => {
  if (!running) {
    return;
  }

  shutdown.abort();
  await Promise.race([prismWorker.close(), new Promise((resolve) => setTimeout(resolve, 5000))]);
};
