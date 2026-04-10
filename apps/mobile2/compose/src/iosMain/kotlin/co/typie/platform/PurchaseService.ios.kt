@file:OptIn(kotlinx.cinterop.ExperimentalForeignApi::class)

package co.typie.platform

import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import swiftPMImport.co.typie.compose.PurchaseEventPayload
import swiftPMImport.co.typie.compose.PurchaseProductPayload
import swiftPMImport.co.typie.compose.SubscriptionPurchaseBridge

internal class IOSPurchaseService : PurchaseService {
  override val events: SharedFlow<PurchaseEvent> =
    MutableSharedFlow(replay = 0, extraBufferCapacity = 8)

  private val mutableEvents: MutableSharedFlow<PurchaseEvent>
    get() = events as MutableSharedFlow<PurchaseEvent>

  private val bridgeMutex = Mutex()
  private var bridge: SubscriptionPurchaseBridge? = null

  private suspend fun bridge(): SubscriptionPurchaseBridge {
    bridge?.let {
      return it
    }

    return bridgeMutex.withLock {
      bridge?.let {
        return it
      }

      withContext(Dispatchers.Main) {
        SubscriptionPurchaseBridge().also { createdBridge ->
          createdBridge.startObservingPurchasesWithCompletion { payload ->
            payload?.toPurchaseEvent()?.let(mutableEvents::tryEmit)
          }
          bridge = createdBridge
        }
      }
    }
  }

  override suspend fun queryProducts(): Map<PurchasePlanInterval, PurchaseProduct> {
    val bridge = bridge()

    return withContext(Dispatchers.Main) {
      suspendCancellableCoroutine { continuation ->
        bridge.queryProductsWithCompletion { payloads, error ->
          if (error != null) {
            continuation.resumeWithException(IllegalStateException(error.toString()))
            return@queryProductsWithCompletion
          }

          val products =
            payloads.orEmpty().filterIsInstance<PurchaseProductPayload>().mapNotNull { payload ->
              payload.toPurchaseProduct()
            }

          continuation.resume(products.associateBy { it.interval })
        }
      }
    }
  }

  override suspend fun purchase(product: PurchaseProduct, accountId: String): Boolean {
    val bridge = bridge()

    return withContext(Dispatchers.Main) {
      suspendCancellableCoroutine { continuation ->
        bridge.purchaseProductWithProductId(productId = product.id, accountId = accountId) {
          success: Boolean,
          _: Any? ->
          continuation.resume(success)
        }
      }
    }
  }

  override suspend fun openSubscriptionManagement(): Boolean {
    val bridge = bridge()

    return withContext(Dispatchers.Main) {
      suspendCancellableCoroutine { continuation ->
        bridge.openSubscriptionManagementWithCompletion { success -> continuation.resume(success) }
      }
    }
  }
}

private fun PurchaseEventPayload.toPurchaseEvent(): PurchaseEvent {
  return when (type) {
    "restored" ->
      PurchaseEvent.Restored(
        productId = productId,
        store = PurchaseStore.AppStore,
        verificationData = verificationData,
        purchaseId = purchaseId,
      )

    else ->
      PurchaseEvent.Purchased(
        productId = productId,
        store = PurchaseStore.AppStore,
        verificationData = verificationData,
        purchaseId = purchaseId,
      )
  }
}

private fun PurchaseProductPayload.toPurchaseProduct(): PurchaseProduct? {
  val interval =
    when (interval) {
      "monthly" -> PurchasePlanInterval.Monthly
      "yearly" -> PurchasePlanInterval.Yearly
      else -> return null
    }

  return PurchaseProduct(
    id = productId,
    interval = interval,
    price = price,
    title = title,
    rawPrice = rawPrice,
    currencyCode = currencyCode,
  )
}
