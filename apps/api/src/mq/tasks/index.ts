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
  SubscriptionReconcileInAppPurchaseCron,
  SubscriptionReconcileInAppPurchaseJob,
  SubscriptionRenewalCancelJob,
  SubscriptionRenewalCron,
  SubscriptionRenewalInitialJob,
  SubscriptionRenewalPlanChangeJob,
  SubscriptionRenewalRetryJob,
  SubscriptionTransitionCron,
} from './subscription.ts';

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
  SendSubscriptionGracePeriodEmailJob,
  SendSubscriptionExpiringEmailJob,
  SendSubscriptionExpiredEmailJob,
  SendSubscriptionWaivedEmailJob,
];

export const crons = [
  DocumentChangesetsScanCron,
  DocumentZombieSweepDueCron,
  SubscriptionRenewalCron,
  SubscriptionTransitionCron,
  SubscriptionReconcileInAppPurchaseCron,
];

export type Jobs = typeof jobs;
export type JobName = Jobs[number]['name'];
export type JobMap = { [Job in Jobs[number] as Job['name']]: Job['fn'] };
