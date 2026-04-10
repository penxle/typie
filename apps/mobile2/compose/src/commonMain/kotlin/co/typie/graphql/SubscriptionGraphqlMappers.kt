package co.typie.graphql

import co.typie.graphql.type.PlanAvailability
import co.typie.graphql.type.SubscriptionState as GraphqlSubscriptionState
import co.typie.service.SubscriptionAvailability
import co.typie.service.SubscriptionSnapshot
import co.typie.service.SubscriptionState

fun MoreScreen_Query.Subscription.toSubscriptionSnapshot(): SubscriptionSnapshot {
  return SubscriptionSnapshot(id = id, planId = plan.id, planName = plan.name)
}

fun SettingsScreen_Query.Subscription.toSubscriptionSnapshot(): SubscriptionSnapshot {
  return SubscriptionSnapshot(id = id)
}

fun FontSettingsScreen_Query.Subscription.toSubscriptionSnapshot(): SubscriptionSnapshot {
  return SubscriptionSnapshot(id = id)
}

fun SpaceSettingsScreen_Query.Subscription.toSubscriptionSnapshot(): SubscriptionSnapshot {
  return SubscriptionSnapshot(id = id)
}

fun EnrollPlanScreen_Query.Subscription.toSubscriptionSnapshot(): SubscriptionSnapshot {
  return SubscriptionSnapshot(
    id = id,
    planId = plan.id,
    availability = plan.availability.toSubscriptionAvailability(),
  )
}

fun CurrentPlanScreen_Query.Subscription.toSubscriptionSnapshot(): SubscriptionSnapshot {
  return SubscriptionSnapshot(
    id = id,
    state = state.toSubscriptionState(),
    startsAt = startsAt,
    expiresAt = expiresAt,
    planId = plan.id,
    planName = plan.name,
    fee = plan.fee,
    availability = plan.availability.toSubscriptionAvailability(),
  )
}

fun CancelPlanScreen_Query.Subscription.toSubscriptionSnapshot(): SubscriptionSnapshot {
  return SubscriptionSnapshot(
    id = id,
    state = state.toSubscriptionState(),
    expiresAt = expiresAt,
    planId = plan.id,
    planName = plan.name,
  )
}

private fun PlanAvailability.toSubscriptionAvailability(): SubscriptionAvailability? {
  return when (this) {
    PlanAvailability.BILLING_KEY -> SubscriptionAvailability.BillingKey
    PlanAvailability.IN_APP_PURCHASE -> SubscriptionAvailability.InAppPurchase
    PlanAvailability.MANUAL -> SubscriptionAvailability.Manual
    PlanAvailability.TRIAL -> SubscriptionAvailability.Trial
    PlanAvailability.UNKNOWN__ -> null
  }
}

private fun GraphqlSubscriptionState.toSubscriptionState(): SubscriptionState {
  return if (this == GraphqlSubscriptionState.ACTIVE) {
    SubscriptionState.Active
  } else {
    SubscriptionState.Canceled
  }
}
