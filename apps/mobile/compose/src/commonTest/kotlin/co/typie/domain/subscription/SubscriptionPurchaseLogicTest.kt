package co.typie.domain.subscription

import co.typie.platform.Platform
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
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
}
