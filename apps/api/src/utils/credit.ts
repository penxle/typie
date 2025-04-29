import dayjs from 'dayjs';
import { and, asc, eq, gt, sum } from 'drizzle-orm';
import { db, UserPaymentCredits } from '@/db';
import type { Transaction } from '@/db';

type GetUserCreditParams = {
  userId: string;
};
export const getUserCredit = async ({ userId }: GetUserCreditParams) => {
  return await db
    .select({ sum: sum(UserPaymentCredits.remainingAmount).mapWith(Number) })
    .from(UserPaymentCredits)
    .where(and(eq(UserPaymentCredits.userId, userId), gt(UserPaymentCredits.remainingAmount, 0), gt(UserPaymentCredits.expiresAt, dayjs())))
    .then(([{ sum }]) => sum ?? 0);
};

type DeductUserCreditParams = {
  tx: Transaction;
  userId: string;
  amount: number;
};
export const deductUserCredit = async ({ tx, userId, amount }: DeductUserCreditParams) => {
  if (amount <= 0) {
    throw new Error('amount must be positive');
  }

  const credits = await tx
    .select({ id: UserPaymentCredits.id, remainingAmount: UserPaymentCredits.remainingAmount })
    .from(UserPaymentCredits)
    .where(and(eq(UserPaymentCredits.userId, userId), gt(UserPaymentCredits.remainingAmount, 0), gt(UserPaymentCredits.expiresAt, dayjs())))
    .orderBy(asc(UserPaymentCredits.expiresAt))
    .for('update');

  let targetAmount = amount;
  const changes: { id: string; remainingAmount: number }[] = [];

  for (const credit of credits) {
    if (targetAmount === 0) {
      break;
    }

    const remainingAmount = Math.max(credit.remainingAmount - targetAmount, 0);
    targetAmount -= credit.remainingAmount - remainingAmount;

    changes.push({
      id: credit.id,
      remainingAmount,
    });
  }

  for (const { id, remainingAmount } of changes) {
    await tx.update(UserPaymentCredits).set({ remainingAmount }).where(eq(UserPaymentCredits.id, id));
  }

  return {
    deductedAmount: amount - targetAmount,
    remainingAmount: targetAmount,
  };
};
