package co.typie.screen.subscription.currentplan

import co.typie.domain.subscription.Subscription
import co.typie.graphql.type.PlanAvailability
import co.typie.graphql.type.SubscriptionState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlin.time.Duration.Companion.days
import kotlin.time.Duration.Companion.hours
import kotlin.time.Instant

class CurrentPlanCopyTest {
  private val now = Instant.fromEpochMilliseconds(1_700_000_000_000)

  private fun subscription(
    state: SubscriptionState = SubscriptionState.ACTIVE,
    currentPeriodEndsAt: Instant = now + 30.days,
    availability: PlanAvailability = PlanAvailability.IN_APP_PURCHASE,
  ) =
    Subscription(
      id = "SUB1",
      state = state,
      startsAt = now - 1.hours,
      currentPeriodEndsAt = currentPeriodEndsAt,
      planId = "pl0fl1map",
      planName = "FULL ACCESS",
      fee = 2900,
      availability = availability,
    )

  @Test
  fun activeShowsNextBillingDate() {
    assertTrue(subscriptionPeriodLine(subscription(), now).startsWith("다음 결제일: "))
  }

  @Test
  fun elapsedActiveDoesNotShowAPastBillingDate() {
    assertEquals(
      "갱신 처리 중",
      subscriptionPeriodLine(subscription(currentPeriodEndsAt = now - 1.hours), now),
    )
  }

  @Test
  fun gracePeriodShowsNoDate() {
    assertEquals(
      "결제를 다시 시도하고 있어요",
      subscriptionPeriodLine(
        subscription(state = SubscriptionState.IN_GRACE_PERIOD, currentPeriodEndsAt = now - 1.days),
        now,
      ),
    )
  }

  @Test
  fun willActivateShowsTransitionInProgress() {
    assertEquals(
      "플랜 전환 처리 중",
      subscriptionPeriodLine(subscription(state = SubscriptionState.WILL_ACTIVATE), now),
    )
  }

  @Test
  fun willExpireShowsCancellationDate() {
    assertTrue(
      subscriptionPeriodLine(subscription(state = SubscriptionState.WILL_EXPIRE), now)
        .startsWith("해지 예정일: ")
    )
  }

  @Test
  fun indefinitePeriodIsNotRenderedAsADate() {
    val sentinel = Instant.parse("9999-12-31T00:00:00Z")
    assertTrue(isIndefinitePeriod(sentinel))
    assertEquals(
      "무기한",
      subscriptionPeriodLine(
        subscription(
          state = SubscriptionState.ACTIVE,
          currentPeriodEndsAt = sentinel,
          availability = PlanAvailability.MANUAL,
        ),
        now,
      ),
    )
  }
}
