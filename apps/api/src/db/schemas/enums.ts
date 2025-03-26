import { pgEnum } from 'drizzle-orm/pg-core';
import * as E from '@/enums';

function createPgEnum<T extends string>(enumName: string, obj: Record<string, T>) {
  return pgEnum(enumName, Object.values(obj) as [T, ...T[]]);
}

export const _JobState = createPgEnum('_job_state', E.JobState);
export const _PostState = createPgEnum('_post_state', E.PostState);
export const _PreorderPaymentState = createPgEnum('_preorder_payment_state', E.PreorderPaymentState);
export const _SingleSignOnProvider = createPgEnum('_single_sign_on_provider', E.SingleSignOnProvider);
export const _UserState = createPgEnum('_user_state', E.UserState);
