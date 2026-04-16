package co.typie.screen.settings.fontsettings

import co.typie.domain.subscription.Subscription
import co.typie.graphql.type.PlanAvailability
import co.typie.graphql.type.SubscriptionState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.time.Clock
import kotlin.time.Duration.Companion.days

private fun mockSubscription() =
  Subscription(
    id = "sub-1",
    state = SubscriptionState.ACTIVE,
    startsAt = Clock.System.now(),
    expiresAt = Clock.System.now() + 30.days,
    planId = "plan-1",
    planName = "FULL ACCESS",
    fee = 12900,
    availability = PlanAvailability.IN_APP_PURCHASE,
  )

class FontSettingsModelsTest {
  @Test
  fun `fontWeightLabel uses shared weight labels before subfamily fallback`() {
    assertEquals("보통", fontWeightLabel(weight = 400, subfamilyDisplayName = "Regular"))
    assertEquals(
      "Semi Condensed (450)",
      fontWeightLabel(weight = 450, subfamilyDisplayName = "Semi Condensed"),
    )
    assertEquals("950", fontWeightLabel(weight = 950, subfamilyDisplayName = null))
  }
}
