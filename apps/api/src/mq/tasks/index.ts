import { SendSubscriptionExpiredEmailJob, SendSubscriptionExpiringEmailJob, SendSubscriptionGracePeriodEmailJob } from './email';
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
  CreateNotificationJob,
  SubscriptionRenewalInitialJob,
  SubscriptionRenewalRetryJob,
  SubscriptionRenewalPlanChangeJob,
  SubscriptionRenewalCancelJob,
  SendSubscriptionGracePeriodEmailJob,
  SendSubscriptionExpiringEmailJob,
  SendSubscriptionExpiredEmailJob,
];

export const crons = [PostCompactScanCron, SubscriptionRenewalCron];

export type Jobs = typeof jobs;
export type JobName = Jobs[number]['name'];
export type JobMap = { [Job in Jobs[number] as Job['name']]: Job['fn'] };
