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

/** 플랜 변경 시 기존 계약을 어떻게 대체할지. 업그레이드는 잔여 가치를 시간으로 환산해 즉시 발효하고, 다운그레이드는 기간이 끝난 뒤 발효한다. */
enum class PurchaseReplacementMode {
  UPGRADE_WITH_TIME_PRORATION,
  DEFERRED,
}

interface PurchaseService {
  val events: SharedFlow<PurchaseEvent>

  suspend fun launch()

  suspend fun queryProducts(productIds: List<String>): List<PurchaseProduct>

  /**
   * [existingPurchaseToken] 과 [replacementMode] 가 있으면 플랜 변경으로 구매를 시작한다 — 그래야 스토어 응답에 승계 포인터가 실려 서버가
   * 독립 토큰으로 거절하지 않는다.
   */
  context(activity: ActivityContext)
  suspend fun purchase(
    product: PurchaseProduct,
    accountId: String,
    existingPurchaseToken: String? = null,
    replacementMode: PurchaseReplacementMode? = null,
  ): Boolean

  /** 현재 보유 중인 구독의 스토어 토큰. 승계 개념이 없는 플랫폼은 null 이다. */
  suspend fun currentSubscriptionPurchaseToken(): String? = null

  suspend fun finishTransaction(subscriptionId: String)
}
