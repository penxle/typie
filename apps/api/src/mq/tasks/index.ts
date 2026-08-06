import {
  DocumentChangesetsCollectJob,
  DocumentChangesetsConsolidateJob,
  DocumentChangesetsScanCron,
  DocumentZombieSweepDueCron,
  DocumentZombieSweepJob,
} from './changeset.ts';
import {
  SendSubscriptionExpiredEmailJob,
  SendSubscriptionExpiringEmailJob,
  SendSubscriptionGracePeriodEmailJob,
  SendSubscriptionWaivedEmailJob,
} from './email.ts';
import { DocumentIndexJob, FolderIndexJob } from './search.ts';
import {
  IapIngestJob,
  IapSyncJob,
  SubscriptionBillingScanCron,
  SubscriptionReconcileInAppPurchaseCron,
  SubscriptionReconcileInAppPurchaseJob,
  SubscriptionRenewalCancelJob,
  SubscriptionRenewalCron,
  SubscriptionRenewalInitialJob,
  SubscriptionRenewalPlanChangeJob,
  SubscriptionRenewalRetryJob,
  SubscriptionTransitionCron,
} from './subscription.ts';
import { SubscriptionInvariantsCron } from './subscription-monitor.ts';

export const jobs = [
  DocumentChangesetsCollectJob,
  DocumentChangesetsConsolidateJob,
  DocumentZombieSweepJob,
  DocumentIndexJob,
  FolderIndexJob,
  SubscriptionRenewalInitialJob,
  SubscriptionRenewalRetryJob,
  SubscriptionRenewalPlanChangeJob,
  SubscriptionRenewalCancelJob,
  SubscriptionReconcileInAppPurchaseJob,
  IapIngestJob,
  IapSyncJob,
  SendSubscriptionGracePeriodEmailJob,
  SendSubscriptionExpiringEmailJob,
  SendSubscriptionExpiredEmailJob,
  SendSubscriptionWaivedEmailJob,
];

export const crons = [
  DocumentChangesetsScanCron,
  DocumentZombieSweepDueCron,
  SubscriptionBillingScanCron,
  SubscriptionRenewalCron,
  SubscriptionTransitionCron,
  SubscriptionReconcileInAppPurchaseCron,
  SubscriptionInvariantsCron,
];

export type Jobs = typeof jobs;
export type JobName = Jobs[number]['name'];
export type JobMap = { [Job in Jobs[number] as Job['name']]: Job['fn'] };
