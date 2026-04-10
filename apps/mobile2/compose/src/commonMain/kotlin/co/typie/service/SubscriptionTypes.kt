package co.typie.service

import co.typie.graphql.QueryState
import co.typie.platform.PurchasePlanInterval
import kotlin.time.Instant

const val FULL_ACCESS_MONTHLY_PLAN_ID = "PL0FL1MAP"
const val FULL_ACCESS_YEARLY_PLAN_ID = "PL0FL1YAP"
const val FULL_ACCESS_MONTHLY_STORE_PRODUCT_ID = "pl0fl1map"
const val FULL_ACCESS_YEARLY_STORE_PRODUCT_ID = "pl0fl1yap"
const val FULL_ACCESS_GOOGLE_PLAY_PRODUCT_ID = "plan.full"
const val TRIAL_START_CONFIRM_TITLE = "무료 체험을 시작하시겠어요?"
const val TRIAL_START_CONFIRM_MESSAGE =
  "결제 수단 등록 없이 2주간 타이피의 모든 기능을 무료로 이용할 수 있어요. 체험 종료 후 자동 결제되지 않아요."
const val TRIAL_START_CONFIRM_ACTION = "시작하기"

data class SubscriptionSnapshot(
  val id: String,
  val state: SubscriptionState? = null,
  val startsAt: Instant? = null,
  val expiresAt: Instant? = null,
  val planId: String? = null,
  val planName: String? = null,
  val fee: Int? = null,
  val availability: SubscriptionAvailability? = null,
)

data class SubscriptionSummary(val hasSubscription: Boolean, val subscriptionName: String)

enum class SubscriptionAvailability {
  InAppPurchase,
  BillingKey,
  Manual,
  Trial,
}

enum class SubscriptionState {
  Active,
  Canceled,
}

fun subscriptionSummary(subscription: SubscriptionSnapshot?): SubscriptionSummary {
  return SubscriptionSummary(
    hasSubscription = subscription != null,
    subscriptionName = subscription?.planName ?: "타이피 BASIC ACCESS",
  )
}

fun QueryState<SubscriptionSnapshot?>.hasSubscriptionOrNull(): Boolean? {
  return when (this) {
    is QueryState.Success -> data != null
    QueryState.Loading,
    is QueryState.Error -> null
  }
}

fun QueryState<SubscriptionSnapshot?>.subscriptionSummaryOrNull(): SubscriptionSummary? {
  return when (this) {
    is QueryState.Success -> subscriptionSummary(data)
    QueryState.Loading,
    is QueryState.Error -> null
  }
}

fun subscriptionPlanId(interval: PurchasePlanInterval): String {
  return when (interval) {
    PurchasePlanInterval.Monthly -> FULL_ACCESS_MONTHLY_PLAN_ID
    PurchasePlanInterval.Yearly -> FULL_ACCESS_YEARLY_PLAN_ID
  }
}

fun isCurrentFullPlan(currentPlanId: String?, interval: PurchasePlanInterval): Boolean {
  return currentPlanId == subscriptionPlanId(interval)
}

fun shouldShowPurchaseCelebration(
  originalSubscriptionId: String?,
  originalPlanId: String?,
  updatedSubscriptionId: String,
  updatedPlanId: String,
): Boolean {
  return originalSubscriptionId != updatedSubscriptionId || originalPlanId != updatedPlanId
}

fun shouldAutoCloseCurrentPlan(state: QueryState<SubscriptionSnapshot?>): Boolean {
  return when (state) {
    is QueryState.Success -> {
      val subscription = state.data ?: return true
      subscription.expiresAt == null
    }
    QueryState.Loading,
    is QueryState.Error -> false
  }
}

fun shouldCloseCancelPlanAfterStoreReturn(
  awaitingStoreResult: Boolean,
  subscriptionState: SubscriptionState?,
): Boolean {
  if (!awaitingStoreResult) {
    return false
  }

  return subscriptionState == null || subscriptionState != SubscriptionState.Active
}

internal fun Int.formatGrouped(): String {
  val text = toString()
  val builder = StringBuilder()

  text.forEachIndexed { index, char ->
    if (index > 0 && (text.length - index) % 3 == 0) {
      builder.append(',')
    }
    builder.append(char)
  }

  return builder.toString()
}
