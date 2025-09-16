import { pgEnum } from 'drizzle-orm/pg-core';
import * as E from '@/enums';

function createPgEnum<T extends string>(enumName: string, obj: Record<string, T>) {
  return pgEnum(enumName, Object.values(obj) as [T, ...T[]]);
}

export const _CommentState = createPgEnum('_comment_state', E.CommentState);
export const _CreditCodeState = createPgEnum('_credit_code_state', E.CreditCodeState);
export const _EntityAvailability = createPgEnum('_entity_availability', E.EntityAvailability);
export const _EntityState = createPgEnum('_entity_state', E.EntityState);
export const _EntityType = createPgEnum('_entity_type', E.EntityType);
export const _EntityVisibility = createPgEnum('_entity_visibility', E.EntityVisibility);
export const _FontState = createPgEnum('_font_state', E.FontState);
export const _FontFamilyState = createPgEnum('_font_family_state', E.FontFamilyState);
export const _InAppPurchaseStore = createPgEnum('_in_app_purchase_store', E.InAppPurchaseStore);
export const _NoteState = createPgEnum('_note_state', E.NoteState);
export const _NotificationState = createPgEnum('_notification_state', E.NotificationState);
export const _PaymentInvoiceState = createPgEnum('_payment_invoice_state', E.PaymentInvoiceState);
export const _PaymentOutcome = createPgEnum('_payment_outcome', E.PaymentOutcome);
export const _PlanAvailability = createPgEnum('_plan_availability', E.PlanAvailability);
export const _PlanInterval = createPgEnum('_plan_interval', E.PlanInterval);
export const _PostContentRating = createPgEnum('_post_content_rating', E.PostContentRating);
export const _PostLayoutMode = createPgEnum('_post_layout_mode', E.PostLayoutMode);
export const _PostType = createPgEnum('_post_type', E.PostType);
export const _PreorderPaymentState = createPgEnum('_preorder_payment_state', E.PreorderPaymentState);
export const _SiteState = createPgEnum('_site_state', E.SiteState);
export const _SingleSignOnProvider = createPgEnum('_single_sign_on_provider', E.SingleSignOnProvider);
export const _SubscriptionState = createPgEnum('_subscription_state', E.SubscriptionState);
export const _UserRole = createPgEnum('_user_role', E.UserRole);
export const _UserState = createPgEnum('_user_state', E.UserState);
