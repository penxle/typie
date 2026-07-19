package co.typie.domain.subscription

import co.typie.graphql.type.PlanAvailability
import co.typie.graphql.type.SubscriptionState
import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlin.time.Duration.Companion.hours
import kotlin.time.Instant

class EditorSubscriptionGateTest {
  private val now = Instant.fromEpochMilliseconds(1_700_000_000_000)

  private val active =
    Entitlement.Active(
      subscription =
        Subscription(
          id = "SUB1",
          state = SubscriptionState.ACTIVE,
          startsAt = now - 1.hours,
          expiresAt = now + 1.hours,
          planId = "PL1",
          planName = "FULL ACCESS",
          fee = 4900,
          availability = PlanAvailability.IN_APP_PURCHASE,
        ),
      inGracePeriod = false,
    )

  @Test
  fun lockedDocumentIsReadOnlyEvenWhenActive() {
    assertTrue(editorIsReadOnly(documentLocked = true, entitlement = active))
  }

  @Test
  fun expiredEntitlementIsReadOnly() {
    assertTrue(editorIsReadOnly(documentLocked = false, entitlement = Entitlement.Expired))
  }

  @Test
  fun activeUnlockedIsEditable() {
    assertFalse(editorIsReadOnly(documentLocked = false, entitlement = active))
  }

  @Test
  fun unknownUnlockedIsEditable() {
    assertFalse(editorIsReadOnly(documentLocked = false, entitlement = Entitlement.Unknown))
  }

  @Test
  fun pushIsBlockedOnlyWhenExpired() {
    assertFalse(shouldAttemptPush(Entitlement.Expired))
    assertTrue(shouldAttemptPush(Entitlement.Unknown))
    assertTrue(shouldAttemptPush(active))
  }
}
