import {
  DocumentGCJob,
  DocumentGCScanCron,
  DocumentPreviewInvalidateJob,
  DocumentSyncCollectJob,
  DocumentSyncScanCron,
} from './document.ts';
import { SendSubscriptionExpiredEmailJob, SendSubscriptionExpiringEmailJob, SendSubscriptionGracePeriodEmailJob } from './email.ts';
import { DocumentIndexJob, FolderIndexJob } from './search.ts';
import {
  SubscriptionRenewalCancelJob,
  SubscriptionRenewalCron,
  SubscriptionRenewalInitialJob,
  SubscriptionRenewalPlanChangeJob,
  SubscriptionRenewalRetryJob,
} from './subscription.ts';

export const jobs = [
  DocumentSyncCollectJob,
  DocumentPreviewInvalidateJob,
  DocumentIndexJob,
  FolderIndexJob,
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
