import dayjs from 'dayjs';
import { eq } from 'drizzle-orm';
import { LogicEngine } from 'json-logic-engine';
import { db, first, Subscriptions, Users } from '@/db';
import type { CouponCondition } from '@/db/schemas/json';

const engine = new LogicEngine();

engine.addMethod('date<', ([a, b]: [unknown, unknown]) => dayjs(a as string).isBefore(dayjs(b as string)));
engine.addMethod('date>', ([a, b]: [unknown, unknown]) => dayjs(a as string).isAfter(dayjs(b as string)));
engine.addMethod('date<=', ([a, b]: [unknown, unknown]) => {
  const da = dayjs(a as string);
  const db = dayjs(b as string);
  return da.isBefore(db) || da.isSame(db);
});
engine.addMethod('date>=', ([a, b]: [unknown, unknown]) => {
  const da = dayjs(a as string);
  const db = dayjs(b as string);
  return da.isAfter(db) || db.isSame(da);
});

export const buildCouponContext = async (userId: string) => {
  const user = await db.select().from(Users).where(eq(Users.id, userId)).then(first);
  const subscriptions = await db.select().from(Subscriptions).where(eq(Subscriptions.userId, userId));

  return {
    user,
    subscriptions,
    now: dayjs(),
  };
};

export const evaluateCouponCondition = async (condition: CouponCondition | null, userId: string): Promise<boolean> => {
  if (!condition) return true;

  const context = await buildCouponContext(userId);
  return engine.run(condition, context) as boolean;
};
