package co.typie.platform

import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow

internal class DesktopPurchaseService : PurchaseService {
  private val _events = MutableSharedFlow<PurchaseEvent>()
  override val events: SharedFlow<PurchaseEvent> = _events

  override suspend fun launch() = Unit

  override suspend fun queryProducts(productIds: List<String>): List<PurchaseProduct> {
    return listOf(
      PurchaseProduct(
        productId = "",
        planId = "pl0fl1map",
        name = "타이피 FULL ACCESS 월간",
        price = "6,900원",
      ),
      PurchaseProduct(
        productId = "",
        planId = "pl0fl1yap",
        name = "타이피 FULL ACCESS 연간",
        price = "69,000원",
      ),
    )
  }

  context(_: ActivityContext)
  override suspend fun purchase(product: PurchaseProduct, accountId: String): Boolean = false

  override suspend fun finishTransaction(subscriptionId: String) = Unit
}
