@file:OptIn(kotlinx.cinterop.ExperimentalForeignApi::class)

package co.typie.platform

import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import swiftPMImport.co.typie.compose.PurchaseEventPayload
import swiftPMImport.co.typie.compose.PurchaseProductPayload
import swiftPMImport.co.typie.compose.SubscriptionPurchaseBridge

internal class IOSPurchaseService : PurchaseService {
  private var bridge: SubscriptionPurchaseBridge? = null

  private val _events = MutableSharedFlow<PurchaseEvent>(extraBufferCapacity = 8)
  override val events: SharedFlow<PurchaseEvent> = _events

  override suspend fun launch() {
    withContext(Dispatchers.Main) {
      val b = SubscriptionPurchaseBridge()
      b.startObservingPurchasesWithCompletion { payload ->
        payload?.toPurchaseEvent()?.let { _events.tryEmit(it) }
      }
      bridge = b
    }
  }

  override suspend fun queryProducts(productIds: List<String>): List<PurchaseProduct> {
    val bridge = bridge ?: return emptyList()

    return withContext(Dispatchers.Main) {
      suspendCancellableCoroutine { continuation ->
        bridge.queryProductsWithProductIds(productIds) { payloads, error ->
          if (error != null) {
            continuation.resumeWithException(IllegalStateException(error.toString()))
            return@queryProductsWithProductIds
          }

          val products =
            payloads.orEmpty().filterIsInstance<PurchaseProductPayload>().map {
              it.toPurchaseProduct()
            }

          continuation.resume(products)
        }
      }
    }
  }

  context(activity: ActivityContext)
  override suspend fun purchase(product: PurchaseProduct, accountId: String): Boolean {
    val bridge = bridge ?: return false

    return withContext(Dispatchers.Main) {
      suspendCancellableCoroutine { continuation ->
        bridge.purchaseProductWithProductId(productId = product.productId, accountId = accountId) {
          success: Boolean,
          _: Any? ->
          continuation.resume(success)
        }
      }
    }
  }

  override suspend fun finishTransaction(subscriptionId: String) {
    val bridge = bridge ?: return

    withContext(Dispatchers.Main) {
      suspendCancellableCoroutine { continuation ->
        bridge.finishTransactionWithTransactionId(subscriptionId) { _ -> continuation.resume(Unit) }
      }
    }
  }
}

private fun PurchaseEventPayload.toPurchaseEvent(): PurchaseEvent {
  return when (type) {
    "restored" ->
      PurchaseEvent(
        kind = PurchaseEventKind.Restored,
        productId = productId,
        subscriptionId = subscriptionId,
        transactionId = transactionId,
      )

    else ->
      PurchaseEvent(
        kind = PurchaseEventKind.Purchased,
        productId = productId,
        subscriptionId = subscriptionId,
        transactionId = transactionId,
      )
  }
}

private fun PurchaseProductPayload.toPurchaseProduct(): PurchaseProduct {
  return PurchaseProduct(productId = productId, planId = productId, name = name, price = price)
}
