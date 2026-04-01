package co.typie.platform

import co.typie.screen.subscription.FULL_ACCESS_MONTHLY_STORE_PRODUCT_ID
import co.typie.screen.subscription.FULL_ACCESS_YEARLY_STORE_PRODUCT_ID
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow

internal class DesktopPurchaseService : PurchaseService {
  override val events: SharedFlow<PurchaseEvent> = MutableSharedFlow(
    replay = 0,
    extraBufferCapacity = 8,
  )

  override suspend fun queryProducts(): Map<PurchasePlanInterval, PurchaseProduct> {
    return mapOf(
      PurchasePlanInterval.Monthly to PurchaseProduct(
        id = FULL_ACCESS_MONTHLY_STORE_PRODUCT_ID,
        interval = PurchasePlanInterval.Monthly,
        price = "월 12,900원",
        title = "타이피 FULL ACCESS 월간",
        rawPrice = 12_900.0,
        currencyCode = "KRW",
      ),
      PurchasePlanInterval.Yearly to PurchaseProduct(
        id = FULL_ACCESS_YEARLY_STORE_PRODUCT_ID,
        interval = PurchasePlanInterval.Yearly,
        price = "연 129,000원",
        title = "타이피 FULL ACCESS 연간",
        rawPrice = 129_000.0,
        currencyCode = "KRW",
      ),
    )
  }

  override suspend fun purchase(
    product: PurchaseProduct,
    accountId: String,
  ): Boolean = false

  override suspend fun openSubscriptionManagement(): Boolean = false
}
