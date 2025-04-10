import { pgEnum } from 'drizzle-orm/pg-core';
import * as E from '@/enums';

function createPgEnum<T extends string>(enumName: string, obj: Record<string, T>) {
  return pgEnum(enumName, Object.values(obj) as [T, ...T[]]);
}

export const _CommentState = createPgEnum('_comment_state', E.CommentState);
export const _EntityState = createPgEnum('_entity_state', E.EntityState);
export const _EntityType = createPgEnum('_entity_type', E.EntityType);
export const _JobState = createPgEnum('_job_state', E.JobState);
export const _PaymentInvoiceState = createPgEnum('_payment_invoice_state', E.PaymentInvoiceState);
export const _PaymentMethodState = createPgEnum('_payment_method_state', E.PaymentMethodState);
export const _PaymentRecordState = createPgEnum('_payment_record_state', E.PaymentRecordState);
export const _PlanAvailability = createPgEnum('_plan_availability', E.PlanAvailability);
export const _PostVisibility = createPgEnum('_post_visibility', E.PostVisibility);
export const _PreorderPaymentState = createPgEnum('_preorder_payment_state', E.PreorderPaymentState);
export const _SiteState = createPgEnum('_site_state', E.SiteState);
export const _SingleSignOnProvider = createPgEnum('_single_sign_on_provider', E.SingleSignOnProvider);
export const _UserPlanBillingCycle = createPgEnum('_user_plan_billing_cycle', E.UserPlanBillingCycle);
export const _UserPlanState = createPgEnum('_user_plan_state', E.UserPlanState);
export const _UserState = createPgEnum('_user_state', E.UserState);
