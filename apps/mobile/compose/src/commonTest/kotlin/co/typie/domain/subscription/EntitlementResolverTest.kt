package co.typie.domain.subscription

import co.typie.graphql.type.PlanAvailability
import co.typie.graphql.type.SubscriptionState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.time.Duration.Companion.hours
import kotlin.time.Instant

class EntitlementResolverTest {
  private val now = Instant.fromEpochMilliseconds(1_700_000_000_000)

  private fun subscription(
    state: SubscriptionState = SubscriptionState.ACTIVE,
    expiresAt: Instant = now + 1.hours,
  ) =
    Subscription(
      id = "SUB1",
      state = state,
      startsAt = now - 1.hours,
      expiresAt = expiresAt,
      planId = "PL1",
      planName = "FULL ACCESS",
      fee = 4900,
      availability = PlanAvailability.IN_APP_PURCHASE,
    )

  @Test
  fun nullSubscriptionIsExpired() {
    assertEquals(Entitlement.Expired, resolveEntitlement(null, now))
  }

  @Test
  fun futureExpiryIsActive() {
    val entitlement = resolveEntitlement(subscription(), now)
    assertIs<Entitlement.Active>(entitlement)
    assertEquals(false, entitlement.inGracePeriod)
  }

  @Test
  fun gracePeriodIsActiveWithFlag() {
    val entitlement =
      resolveEntitlement(subscription(state = SubscriptionState.IN_GRACE_PERIOD), now)
    assertIs<Entitlement.Active>(entitlement)
    assertEquals(true, entitlement.inGracePeriod)
  }

  @Test
  fun pastExpiryIsExpiredEvenWithStaleActiveState() {
    assertEquals(
      Entitlement.Expired,
      resolveEntitlement(subscription(expiresAt = now - 1.hours), now),
    )
  }

  @Test
  fun expiryAtExactNowIsExpired() {
    assertEquals(Entitlement.Expired, resolveEntitlement(subscription(expiresAt = now), now))
  }
}
