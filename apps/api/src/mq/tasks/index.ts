import { ProcessBmoMentionJob } from './bmo';
import { DocumentGCJob, DocumentGCScanCron, DocumentIndexJob, DocumentSyncCollectJob, DocumentSyncScanCron } from './document';
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
  DocumentSyncCollectJob,
  DocumentIndexJob,
  DocumentGCJob,
  SubscriptionRenewalInitialJob,
  SubscriptionRenewalRetryJob,
  SubscriptionRenewalPlanChangeJob,
  SubscriptionRenewalCancelJob,
  SendSubscriptionGracePeriodEmailJob,
  SendSubscriptionExpiringEmailJob,
  SendSubscriptionExpiredEmailJob,
  ProcessBmoMentionJob,
];

export const crons = [PostSyncScanCron, PostCompactScanCron, DocumentSyncScanCron, DocumentGCScanCron, SubscriptionRenewalCron];

export type Jobs = typeof jobs;
export type JobName = Jobs[number]['name'];
export type JobMap = { [Job in Jobs[number] as Job['name']]: Job['fn'] };
