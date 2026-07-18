import { eq } from 'drizzle-orm';
import { db, firstOrThrowWith, UserDevices, Users } from '#/db/index.ts';

export const SYSTEM_USER_ID = 'U0SYSTEM000000000';
export const SYSTEM_DEVICE_ID = 'UDEV0SYSTEM000000';

export const ensureSystemActor = async (): Promise<void> => {
  const missing = new Error('system actor missing — run the seed migration first');

  await db.select({ id: Users.id }).from(Users).where(eq(Users.id, SYSTEM_USER_ID)).then(firstOrThrowWith(missing));
  await db.select({ id: UserDevices.id }).from(UserDevices).where(eq(UserDevices.id, SYSTEM_DEVICE_ID)).then(firstOrThrowWith(missing));
};
