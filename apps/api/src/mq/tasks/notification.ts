import { db, Notifications } from '@/db';
import { defineJob } from '../types';
import type { NotificationData } from '@/db/schemas/json';

type CreateNotificationJobParams = { userId: string; data: NotificationData };
export const CreateNotificationJob = defineJob('notification:create', async ({ userId, data }: CreateNotificationJobParams) => {
  // TODO: 알림설정 기능 구현되면 알림 받는지 여부 체크

  await db.insert(Notifications).values({ userId, data });
});
