import { CreateNotificationJob } from './notification';
import { PostSyncCollectJob } from './post';
import { TestCron } from './test';

export const jobs = [PostSyncCollectJob, CreateNotificationJob];
export const crons = [TestCron];

export type Jobs = typeof jobs;
export type JobName = Jobs[number]['name'];
export type JobMap = { [Job in Jobs[number] as Job['name']]: Job['fn'] };
