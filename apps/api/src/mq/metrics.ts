import { metrics, ValueType } from '@opentelemetry/api';
import { queue, worker } from './bullmq';

const meter = metrics.getMeter('bullmq');

export const jobCompletedCounter = meter.createCounter('bullmq.job.completed.total', {
  description: 'Total number of completed jobs by job name',
  unit: 'count',
  valueType: ValueType.INT,
});

export const jobFailedCounter = meter.createCounter('bullmq.job.failed.total', {
  description: 'Total number of failed jobs by job name',
  unit: 'count',
  valueType: ValueType.INT,
});

export const jobStartedCounter = meter.createCounter('bullmq.job.started.total', {
  description: 'Total number of started jobs by job name',
  unit: 'count',
  valueType: ValueType.INT,
});

export const jobDurationHistogram = meter.createHistogram('bullmq.job.duration.milliseconds', {
  description: 'Job processing duration in milliseconds by job name',
  unit: 'milliseconds',
  valueType: ValueType.INT,
});

export const jobWaitTimeHistogram = meter.createHistogram('bullmq.job.wait.milliseconds', {
  description: 'Job wait time from enqueue to execution in milliseconds by job name',
  unit: 'milliseconds',
  valueType: ValueType.INT,
});

export const queueWaitingGauge = meter.createObservableGauge('bullmq.queue.waiting', {
  description: 'Number of jobs waiting in the queue',
  unit: 'count',
  valueType: ValueType.INT,
});

queueWaitingGauge.addCallback(async (result) => {
  const counts = await queue.getJobCounts();
  result.observe(counts.waiting || 0);
});

export const queueActiveGauge = meter.createObservableGauge('bullmq.queue.active', {
  description: 'Number of jobs currently being processed',
  unit: 'count',
  valueType: ValueType.INT,
});

queueActiveGauge.addCallback(async (result) => {
  const counts = await queue.getJobCounts();
  result.observe(counts.active || 0);
});

export const queueDelayedGauge = meter.createObservableGauge('bullmq.queue.delayed', {
  description: 'Number of delayed jobs in the queue',
  unit: 'count',
  valueType: ValueType.INT,
});

queueDelayedGauge.addCallback(async (result) => {
  const counts = await queue.getJobCounts();
  result.observe(counts.delayed || 0);
});

export const queueFailedGauge = meter.createObservableGauge('bullmq.queue.failed', {
  description: 'Number of failed jobs in the queue',
  unit: 'count',
  valueType: ValueType.INT,
});

queueFailedGauge.addCallback(async (result) => {
  const counts = await queue.getJobCounts();
  result.observe(counts.failed || 0);
});

worker.on('active', (job) => {
  jobStartedCounter.add(1, { job_name: job.name });

  if (job.timestamp) {
    const waitTime = Date.now() - job.timestamp;
    jobWaitTimeHistogram.record(waitTime, { job_name: job.name });
  }
});

worker.on('completed', (job) => {
  if (job.processedOn && job.finishedOn) {
    const duration = job.finishedOn - job.processedOn;
    jobDurationHistogram.record(duration, { job_name: job.name });
  }

  jobCompletedCounter.add(1, { job_name: job.name });
});

worker.on('failed', (job) => {
  if (job) {
    jobFailedCounter.add(1, { job_name: job.name });
  }
});
