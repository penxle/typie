import { Queue } from 'bullmq';
import { Redis } from 'ioredis';
import { env, lane } from '#/env.ts';
import { logKeyOf } from '#/utils/prism-ingest-core.ts';
import type { IngestTarget } from '#/utils/prism-ingest-core.ts';

export const PRISM_LANE = `${lane}-prism`;
export const PRISM_INGEST_JOB = 'prism:ingest';

export const prismRedis = () => new Redis({ host: env.REDIS_URL, tls: {}, maxRetriesPerRequest: null });

export const prismQueue = new Queue<{ logKey: string }>(PRISM_LANE, {
  connection: prismRedis(),

  defaultJobOptions: {
    removeOnComplete: true,
    removeOnFail: true,

    attempts: 5,
    backoff: {
      type: 'exponential',
      delay: 1000,
    },
  },
});

export const shutdown = new AbortController();

export const ensureIngest = async (target: IngestTarget): Promise<void> => {
  const logKey = logKeyOf(target);
  await prismQueue.add(PRISM_INGEST_JOB, { logKey }, { jobId: logKey });
};
