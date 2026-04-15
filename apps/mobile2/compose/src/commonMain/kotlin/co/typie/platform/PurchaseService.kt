package co.typie.platform

import kotlinx.coroutines.flow.SharedFlow

open class PurchaseProduct(
  /** 스토어별 구독 상품 ID (Android: `plan.full`, iOS: `pl0fl1map` & `pl0fl1yap` */
  val productId: String,
  /** 구독 상품 내 플랜 ID (Android: `pl0fl1map` & `pl0fl1yap`), iOS: 동일 */
  val planId: String,
  val name: String,
  val price: String,
)

enum class PurchaseEventKind {
  Purchased,
  Restored,
}

data class PurchaseEvent(
  val kind: PurchaseEventKind,
  /** 스토어별 구독 상품 ID (PurchaseProduct.productId 에 대응) */
  val productId: String,
  /** 특정 구독에 대한 고유 ID (구독 갱신해도 변하지 않음) - Android: `purchaseToken`, iOS: `transaction.originalID` */
  val subscriptionId: String,
  /** 특정 결제에 대한 고유 ID (구독 갱신시 매번 변함) - Android: `orderId`, iOS: `transaction.id` */
  val transactionId: String?,
)

interface PurchaseService {
  val events: SharedFlow<PurchaseEvent>

  suspend fun launch()

  suspend fun queryProducts(productIds: List<String>): List<PurchaseProduct>

  context(activity: ActivityContext)
  suspend fun purchase(product: PurchaseProduct, accountId: String): Boolean

  suspend fun finishTransaction(subscriptionId: String)
}
