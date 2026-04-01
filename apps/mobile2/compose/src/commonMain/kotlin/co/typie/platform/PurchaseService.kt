package co.typie.platform

import kotlinx.coroutines.flow.SharedFlow

enum class PurchasePlanInterval {
  Monthly,
  Yearly,
}

enum class PurchaseStore {
  AppStore,
  GooglePlay,
}

data class PurchaseProduct(
  val id: String,
  val interval: PurchasePlanInterval,
  val price: String,
  val title: String? = null,
  val rawPrice: Double? = null,
  val currencyCode: String? = null,
)

sealed interface PurchaseEvent {
  val productId: String
  val store: PurchaseStore

  data class Purchased(
    override val productId: String,
    override val store: PurchaseStore,
    val verificationData: String,
    val purchaseId: String? = null,
  ) : PurchaseEvent

  data class Restored(
    override val productId: String,
    override val store: PurchaseStore,
    val verificationData: String,
    val purchaseId: String? = null,
  ) : PurchaseEvent
}

interface PurchaseService {
  val events: SharedFlow<PurchaseEvent>

  suspend fun queryProducts(): Map<PurchasePlanInterval, PurchaseProduct>

  suspend fun purchase(
    product: PurchaseProduct,
    accountId: String,
  ): Boolean

  suspend fun openSubscriptionManagement(): Boolean
}
