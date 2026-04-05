package co.typie.screen.subscription

import co.typie.graphql.CancelPlanScreen_Query
import co.typie.graphql.CurrentPlanScreen_Query
import co.typie.graphql.EnrollPlanScreen_Query
import co.typie.graphql.FontSettingsScreen_Query
import co.typie.graphql.MoreScreen_Query
import co.typie.graphql.SettingsScreen_Query
import co.typie.graphql.SpaceSettingsScreen_Query
import co.typie.graphql.type.PlanAvailability
import co.typie.graphql.type.SubscriptionState as GraphqlSubscriptionState

fun MoreScreen_Query.Subscription.toSubscriptionSnapshot(): SubscriptionSnapshot {
  return SubscriptionSnapshot(
    id = id,
    planId = plan.id,
    planName = plan.name,
  )
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
