package co.typie.subscription

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

const val FULL_ACCESS_MONTHLY_PLAN_ID = "PL0FL1MAP"
const val FULL_ACCESS_YEARLY_PLAN_ID = "PL0FL1YAP"
const val FULL_ACCESS_MONTHLY_STORE_PRODUCT_ID = "pl0fl1map"
const val FULL_ACCESS_YEARLY_STORE_PRODUCT_ID = "pl0fl1yap"
const val FULL_ACCESS_GOOGLE_PLAY_PRODUCT_ID = "plan.full"
