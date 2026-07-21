import {
  DocumentChangesetsCollectJob,
  DocumentChangesetsConsolidateJob,
  DocumentChangesetsScanCron,
  DocumentZombieSweepDueCron,
  DocumentZombieSweepJob,
} from './changeset.ts';
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
  SubscriptionReconcileInAppPurchaseCron,
  SubscriptionReconcileInAppPurchaseJob,
  SubscriptionRenewalCancelJob,
  SubscriptionRenewalCron,
  SubscriptionRenewalInitialJob,
  SubscriptionRenewalPlanChangeJob,
  SubscriptionRenewalRetryJob,
} from './subscription.ts';

export const jobs = [
  DocumentChangesetsCollectJob,
  DocumentChangesetsConsolidateJob,
  DocumentZombieSweepJob,
  DocumentSyncCollectJob,
  DocumentPreviewInvalidateJob,
  DocumentIndexJob,
  FolderIndexJob,
  DocumentGCJob,
  SubscriptionRenewalInitialJob,
  SubscriptionRenewalRetryJob,
  SubscriptionRenewalPlanChangeJob,
  SubscriptionRenewalCancelJob,
  SubscriptionReconcileInAppPurchaseJob,
  SendSubscriptionGracePeriodEmailJob,
  SendSubscriptionExpiringEmailJob,
  SendSubscriptionExpiredEmailJob,
  SendSubscriptionWaivedEmailJob,
];

export const crons = [
  DocumentChangesetsScanCron,
  DocumentZombieSweepDueCron,
  DocumentSyncScanCron,
  DocumentGCScanCron,
  SubscriptionRenewalCron,
  SubscriptionReconcileInAppPurchaseCron,
];

export type Jobs = typeof jobs;
export type JobName = Jobs[number]['name'];
export type JobMap = { [Job in Jobs[number] as Job['name']]: Job['fn'] };
