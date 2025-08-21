import { queue } from './bullmq';
import type { JobsOptions } from 'bullmq';
import type { JobMap, JobName } from './tasks';
import type { JobFn } from './types';

export const enqueueJob = async <N extends JobName, F extends JobMap[N]>(
  name: N,
  payload: F extends JobFn<infer P> ? P : never,
  options?: JobsOptions,
) => {
  await queue.add(name, payload, options);
};
