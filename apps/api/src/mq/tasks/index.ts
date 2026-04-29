import {
  DocumentGCJob,
  DocumentGCScanCron,
  DocumentPreviewInvalidateJob,
  DocumentSyncCollectJob,
  DocumentSyncScanCron,
} from './document.ts';
import {
  SendSubscriptionExpiredEmailJob,
  SendSubscriptionExpiringEmailJob,
  SendSubscriptionGracePeriodEmailJob,
  SendSubscriptionWaivedEmailJob,
} from './email.ts';
import { DocumentIndexJob, FolderIndexJob } from './search.ts';
import {
  SubscriptionRenewalCancelJob,
  SubscriptionRenewalCron,
  SubscriptionRenewalInitialJob,
  SubscriptionRenewalPlanChangeJob,
  SubscriptionRenewalRetryJob,
} from './subscription.ts';
import { DocumentAdvanceHeadJob, DocumentAdvanceHeadScanCron } from './sync.ts';

export const jobs = [
  DocumentSyncCollectJob,
  DocumentPreviewInvalidateJob,
  DocumentIndexJob,
  FolderIndexJob,
  DocumentGCJob,
  DocumentAdvanceHeadJob,
  SubscriptionRenewalInitialJob,
  SubscriptionRenewalRetryJob,
  SubscriptionRenewalPlanChangeJob,
  SubscriptionRenewalCancelJob,
  SendSubscriptionGracePeriodEmailJob,
  SendSubscriptionExpiringEmailJob,
  SendSubscriptionExpiredEmailJob,
  SendSubscriptionWaivedEmailJob,
];

export const crons = [DocumentSyncScanCron, DocumentGCScanCron, DocumentAdvanceHeadScanCron, SubscriptionRenewalCron];

export type Jobs = typeof jobs;
export type JobName = Jobs[number]['name'];
export type JobMap = { [Job in Jobs[number] as Job['name']]: Job['fn'] };
