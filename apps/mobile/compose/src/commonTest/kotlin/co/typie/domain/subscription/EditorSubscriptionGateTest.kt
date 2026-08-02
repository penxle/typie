package co.typie.domain.subscription

import co.typie.graphql.type.PlanAvailability
import co.typie.graphql.type.SubscriptionState
import kotlin.test.Test
import kotlin.test.assertEquals
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
          currentPeriodEndsAt = now + 1.hours,
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

  @Test
  fun gatesBlockOnlyExpired() {
    // 모든 접근 게이트가 이 술어 하나를 거친다 — Expired 만 차단하고 Unknown 은 열어 둔다(fail-open).
    assertTrue(Entitlement.Unknown.grantsAccess())
    assertTrue(active.grantsAccess())
    assertFalse(Entitlement.Expired.grantsAccess())

    for (entitlement in listOf(Entitlement.Unknown, active, Entitlement.Expired)) {
      val allowed = entitlement.grantsAccess()
      assertEquals(!allowed, editorIsReadOnly(documentLocked = false, entitlement = entitlement))
      assertEquals(allowed, shouldAttemptPush(entitlement))
    }
  }
}
