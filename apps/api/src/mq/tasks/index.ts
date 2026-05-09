import { DocumentChangesetsCollectJob, DocumentChangesetsScanCron } from './changeset.ts';
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

export const jobs = [
  DocumentChangesetsCollectJob,
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
  SendSubscriptionWaivedEmailJob,
];

export const crons = [DocumentChangesetsScanCron, DocumentSyncScanCron, DocumentGCScanCron, SubscriptionRenewalCron];

export type Jobs = typeof jobs;
export type JobName = Jobs[number]['name'];
export type JobMap = { [Job in Jobs[number] as Job['name']]: Job['fn'] };
