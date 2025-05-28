import { CreateNotificationJob } from './notification';
import { PaymentCron } from './payment';
import { PostSyncCollectJob } from './post';

export const jobs = [PostSyncCollectJob, CreateNotificationJob];
export const crons = [PaymentCron];

export type Jobs = typeof jobs;
export type JobName = Jobs[number]['name'];
export type JobMap = { [Job in Jobs[number] as Job['name']]: Job['fn'] };
