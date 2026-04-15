package co.typie.domain.subscription

import co.typie.graphql.type.PlanAvailability
import co.typie.graphql.type.SubscriptionState
import kotlin.time.Instant

data class Subscription(
  val id: String,
  val state: SubscriptionState,
  val startsAt: Instant,
  val expiresAt: Instant,
  val planId: String,
  val planName: String,
  val fee: Int,
  val availability: PlanAvailability,
)
