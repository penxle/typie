package co.typie.domain.subscription

import co.typie.platform.Platform
import co.typie.platform.PurchaseReplacementMode
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class SubscriptionPurchaseLogicTest {
  @Test
  fun androidUsesSingleStoreProduct() {
    assertEquals(listOf("plan.full"), storeProductIds(Platform.Android))
  }

  @Test
  fun iosUsesPlanProducts() {
    assertEquals(listOf("pl0fl1map", "pl0fl1yap"), storeProductIds(Platform.iOS))
  }

  @Test
  fun firstSubscriptionIsNew() {
    assertTrue(isNewSubscription(null, "SUB1"))
  }

  @Test
  fun changedSubscriptionIsNew() {
    assertTrue(isNewSubscription("SUB1", "SUB2"))
  }

  @Test
  fun renewalIsNotNew() {
    assertFalse(isNewSubscription("SUB1", "SUB1"))
  }

  // 현재 플랜은 서버(대문자), 새 플랜은 스토어(소문자)에서 온다 — 같은 플랜이 다른 문자열로 들어온다.
  @Test
  fun monthlyToYearlyIsProratedUpgrade() {
    assertEquals(
      PurchaseReplacementMode.UPGRADE_WITH_TIME_PRORATION,
      planReplacementMode(currentPlanId = "PL0FL1MAP", newPlanId = "pl0fl1yap"),
    )
  }

  @Test
  fun yearlyToMonthlyIsDeferred() {
    assertEquals(
      PurchaseReplacementMode.DEFERRED,
      planReplacementMode(currentPlanId = "PL0FL1YAP", newPlanId = "pl0fl1map"),
    )
  }

  @Test
  fun samePlanIsNotAReplacement() {
    assertNull(planReplacementMode(currentPlanId = "PL0FL1MAP", newPlanId = "pl0fl1map"))
    assertNull(planReplacementMode(currentPlanId = "pl0fl1yap", newPlanId = "pl0fl1yap"))
  }
}
