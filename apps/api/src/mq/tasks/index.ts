import { ProcessBmoMentionJob } from './bmo';
import { CanvasCompactJob, CanvasCompactScanCron, CanvasIndexJob, CanvasSyncCollectJob, CanvasSyncScanCron } from './canvas';
import { SendSubscriptionExpiredEmailJob, SendSubscriptionExpiringEmailJob, SendSubscriptionGracePeriodEmailJob } from './email';
import { PostCompactJob, PostCompactScanCron, PostIndexJob, PostSyncCollectJob, PostSyncScanCron } from './post';
import {
  SubscriptionRenewalCancelJob,
  SubscriptionRenewalCron,
  SubscriptionRenewalInitialJob,
  SubscriptionRenewalPlanChangeJob,
  SubscriptionRenewalRetryJob,
} from './subscription';

export const jobs = [
  PostIndexJob,
  PostSyncCollectJob,
  PostCompactJob,
  CanvasIndexJob,
  CanvasSyncCollectJob,
  CanvasCompactJob,
  SubscriptionRenewalInitialJob,
  SubscriptionRenewalRetryJob,
  SubscriptionRenewalPlanChangeJob,
  SubscriptionRenewalCancelJob,
  SendSubscriptionGracePeriodEmailJob,
  SendSubscriptionExpiringEmailJob,
  SendSubscriptionExpiredEmailJob,
  ProcessBmoMentionJob,
];

export const crons = [PostSyncScanCron, PostCompactScanCron, CanvasSyncScanCron, CanvasCompactScanCron, SubscriptionRenewalCron];

export type Jobs = typeof jobs;
export type JobName = Jobs[number]['name'];
export type JobMap = { [Job in Jobs[number] as Job['name']]: Job['fn'] };
