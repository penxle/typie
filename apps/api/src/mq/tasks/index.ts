import { ProcessBmoMentionJob } from './bmo';
import { CanvasCompactJob, CanvasCompactScanCron, CanvasIndexJob, CanvasSyncCollectJob } from './canvas';
import { SendSubscriptionExpiredEmailJob, SendSubscriptionExpiringEmailJob, SendSubscriptionGracePeriodEmailJob } from './email';
import { DailyAmazingFactJob, GirCron, ProcessGirMentionJob } from './gir';
import { CreateNotificationJob } from './notification';
import { PostCompactJob, PostCompactScanCron, PostIndexJob, PostSyncCollectJob } from './post';
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
  CreateNotificationJob,
  SubscriptionRenewalInitialJob,
  SubscriptionRenewalRetryJob,
  SubscriptionRenewalPlanChangeJob,
  SubscriptionRenewalCancelJob,
  SendSubscriptionGracePeriodEmailJob,
  SendSubscriptionExpiringEmailJob,
  SendSubscriptionExpiredEmailJob,
  ProcessBmoMentionJob,
  ProcessGirMentionJob,
  DailyAmazingFactJob,
];

export const crons = [PostCompactScanCron, CanvasCompactScanCron, SubscriptionRenewalCron, GirCron];

export type Jobs = typeof jobs;
export type JobName = Jobs[number]['name'];
export type JobMap = { [Job in Jobs[number] as Job['name']]: Job['fn'] };
