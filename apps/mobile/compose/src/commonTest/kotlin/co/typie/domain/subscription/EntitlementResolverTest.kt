package co.typie.domain.subscription

import co.typie.graphql.type.PlanAvailability
import co.typie.graphql.type.SubscriptionState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.time.Duration.Companion.days
import kotlin.time.Duration.Companion.hours
import kotlin.time.Instant

class EntitlementResolverTest {
  private val now = Instant.fromEpochMilliseconds(1_700_000_000_000)

  private fun subscription(
    state: SubscriptionState = SubscriptionState.ACTIVE,
    expiresAt: Instant = now + 1.hours,
    availability: PlanAvailability = PlanAvailability.IN_APP_PURCHASE,
  ) =
    Subscription(
      id = "SUB1",
      state = state,
      startsAt = now - 1.hours,
      expiresAt = expiresAt,
      planId = "PL1",
      planName = "FULL ACCESS",
      fee = 4900,
      availability = availability,
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
  fun gracePeriodWithPastExpiryIsActive() {
    val entitlement =
      resolveEntitlement(
        subscription(state = SubscriptionState.IN_GRACE_PERIOD, expiresAt = now - 1.hours),
        now,
      )
    assertIs<Entitlement.Active>(entitlement)
    assertEquals(true, entitlement.inGracePeriod)
  }

  @Test
  fun gracePeriodWithinMaxDurationIsActive() {
    val entitlement =
      resolveEntitlement(
        subscription(state = SubscriptionState.IN_GRACE_PERIOD, expiresAt = now - 25.days),
        now,
      )
    assertIs<Entitlement.Active>(entitlement)
    assertEquals(true, entitlement.inGracePeriod)
  }

  @Test
  fun gracePeriodBeyondMaxDurationIsExpired() {
    assertEquals(
      Entitlement.Expired,
      resolveEntitlement(
        subscription(state = SubscriptionState.IN_GRACE_PERIOD, expiresAt = now - 40.days),
        now,
      ),
    )
  }

  @Test
  fun billingKeyGraceWithinPolicyIsActive() {
    val entitlement =
      resolveEntitlement(
        subscription(
          state = SubscriptionState.IN_GRACE_PERIOD,
          expiresAt = now - 5.days,
          availability = PlanAvailability.BILLING_KEY,
        ),
        now,
      )
    assertIs<Entitlement.Active>(entitlement)
    assertEquals(true, entitlement.inGracePeriod)
  }

  @Test
  fun billingKeyGraceBeyondPolicyIsExpired() {
    assertEquals(
      Entitlement.Expired,
      resolveEntitlement(
        subscription(
          state = SubscriptionState.IN_GRACE_PERIOD,
          expiresAt = now - 10.days,
          availability = PlanAvailability.BILLING_KEY,
        ),
        now,
      ),
    )
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

  @Test
  fun deadlineIsExpiryOutsideGrace() {
    val sub = subscription()
    assertEquals(sub.expiresAt, entitlementDeadline(sub))
  }

  @Test
  fun deadlineIsGraceBoundInGrace() {
    val storeSub =
      subscription(state = SubscriptionState.IN_GRACE_PERIOD, expiresAt = now - 1.hours)
    assertEquals(storeSub.expiresAt + 31.days, entitlementDeadline(storeSub))

    val billingKeySub =
      subscription(
        state = SubscriptionState.IN_GRACE_PERIOD,
        expiresAt = now - 1.hours,
        availability = PlanAvailability.BILLING_KEY,
      )
    assertEquals(billingKeySub.expiresAt + 7.days, entitlementDeadline(billingKeySub))
  }

  @Test
  fun entitlementFlipsAcrossDeadline() {
    // 틱 루프가 이 마감에 예약되므로, 마감 전후로 판정이 실제로 뒤집혀야 오프라인 강등이 성립한다.
    val subs =
      listOf(
        subscription(),
        subscription(state = SubscriptionState.IN_GRACE_PERIOD, expiresAt = now - 1.hours),
        subscription(
          state = SubscriptionState.IN_GRACE_PERIOD,
          expiresAt = now - 1.hours,
          availability = PlanAvailability.BILLING_KEY,
        ),
      )
    for (sub in subs) {
      val deadline = entitlementDeadline(sub)
      assertIs<Entitlement.Active>(resolveEntitlement(sub, deadline - 1.hours))
      assertEquals(Entitlement.Expired, resolveEntitlement(sub, deadline + 1.hours))
    }
  }
}
