import { ProcessBmoMentionJob } from './bmo';
import { CanvasCompactJob, CanvasCompactScanCron, CanvasIndexJob, CanvasSyncCollectJob } from './canvas';
import { SendSubscriptionExpiredEmailJob, SendSubscriptionExpiringEmailJob, SendSubscriptionGracePeriodEmailJob } from './email';
import { CreateNotificationJob } from './notification';
import { PostCompactJob, PostCompactScanCron, PostIndexJob, PostMigrateSnapshotsJob, PostSyncCollectJob } from './post';
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
  PostMigrateSnapshotsJob,
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
];

export const crons = [PostCompactScanCron, CanvasCompactScanCron, SubscriptionRenewalCron];

export type Jobs = typeof jobs;
export type JobName = Jobs[number]['name'];
export type JobMap = { [Job in Jobs[number] as Job['name']]: Job['fn'] };
