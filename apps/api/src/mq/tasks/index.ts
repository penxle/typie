import { DocumentGCJob, DocumentGCScanCron, DocumentIndexJob, DocumentSyncCollectJob, DocumentSyncScanCron } from './document';
import { SendSubscriptionExpiredEmailJob, SendSubscriptionExpiringEmailJob, SendSubscriptionGracePeriodEmailJob } from './email';
import {
  SubscriptionRenewalCancelJob,
  SubscriptionRenewalCron,
  SubscriptionRenewalInitialJob,
  SubscriptionRenewalPlanChangeJob,
  SubscriptionRenewalRetryJob,
} from './subscription';

export const jobs = [
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
];

export const crons = [DocumentSyncScanCron, DocumentGCScanCron, SubscriptionRenewalCron];

export type Jobs = typeof jobs;
export type JobName = Jobs[number]['name'];
export type JobMap = { [Job in Jobs[number] as Job['name']]: Job['fn'] };
