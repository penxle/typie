import { register } from 'tsx/esm/api';
import type { Job } from 'bullmq';

register();

const { jobs, crons } = await import('./tasks');
const taskMap = Object.fromEntries([...jobs, ...crons].map((job) => [job.name, job.fn]));

// eslint-disable-next-line unicorn/no-anonymous-default-export, import/no-default-export
export default async (job: Job) => {
  const fn = taskMap[job.name];
  await fn?.(job.data);
};
