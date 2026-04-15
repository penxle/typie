package co.typie.platform

import android.content.Context
import com.android.billingclient.api.AcknowledgePurchaseParams
import com.android.billingclient.api.BillingClient
import com.android.billingclient.api.BillingClient.BillingResponseCode
import com.android.billingclient.api.BillingClient.ProductType
import com.android.billingclient.api.BillingClientStateListener
import com.android.billingclient.api.BillingFlowParams
import com.android.billingclient.api.BillingResult
import com.android.billingclient.api.PendingPurchasesParams
import com.android.billingclient.api.ProductDetails
import com.android.billingclient.api.Purchase
import com.android.billingclient.api.QueryProductDetailsParams
import kotlin.collections.mapNotNull
import kotlin.collections.orEmpty
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

class AndroidPurchaseProduct(
  productId: String,
  planId: String,
  name: String,
  price: String,
  val productDetails: ProductDetails,
  val offerToken: String,
) : PurchaseProduct(productId, planId, name, price)

internal class AndroidPurchaseService(private val context: Context) : PurchaseService {
  private val coroutineScope = CoroutineScope(Dispatchers.Main + SupervisorJob())
  private val mutex = Mutex()

  private val billingClient =
    BillingClient.newBuilder(context)
      .setListener { _, purchases ->
        purchases?.forEach { purchase -> coroutineScope.launch { handlePurchase(purchase) } }
      }
      .enablePendingPurchases(PendingPurchasesParams.newBuilder().enablePrepaidPlans().build())
      .enableAutoServiceReconnection()
      .build()

  private val _events = MutableSharedFlow<PurchaseEvent>(extraBufferCapacity = 8)
  override val events: SharedFlow<PurchaseEvent> = _events

  override suspend fun launch() {
    ensureConnected()
  }

  override suspend fun queryProducts(productIds: List<String>): List<PurchaseProduct> {
    ensureConnected()

    val params =
      QueryProductDetailsParams.newBuilder()
        .setProductList(
          productIds.map {
            QueryProductDetailsParams.Product.newBuilder()
              .setProductId(it)
              .setProductType(ProductType.SUBS)
              .build()
          }
        )
        .build()

    return suspendCancellableCoroutine { continuation ->
      billingClient.queryProductDetailsAsync(params) { billingResult, queryResult ->
        if (billingResult.responseCode != BillingResponseCode.OK) {
          continuation.resumeWithException(
            IllegalStateException(
              billingResult.debugMessage.ifBlank {
                "Failed to query Google Play subscription products"
              }
            )
          )

          return@queryProductDetailsAsync
        }

        val products = queryResult.productDetailsList.toPurchaseProducts()
        continuation.resume(products)
      }
    }
  }

  context(activity: ActivityContext)
  override suspend fun purchase(product: PurchaseProduct, accountId: String): Boolean {
    ensureConnected()

    val androidProduct = product as? AndroidPurchaseProduct ?: return false

    val params =
      BillingFlowParams.newBuilder()
        .setProductDetailsParamsList(
          listOf(
            BillingFlowParams.ProductDetailsParams.newBuilder()
              .setProductDetails(androidProduct.productDetails)
              .setOfferToken(androidProduct.offerToken)
              .build()
          )
        )
        .setObfuscatedAccountId(accountId)
        .build()

    val billingResult = billingClient.launchBillingFlow(activity, params)
    return billingResult.responseCode == BillingResponseCode.OK
  }

  override suspend fun finishTransaction(subscriptionId: String) {
    val params = AcknowledgePurchaseParams.newBuilder().setPurchaseToken(subscriptionId).build()

    suspendCancellableCoroutine { continuation ->
      billingClient.acknowledgePurchase(params) { continuation.resume(Unit) }
    }
  }

  private suspend fun ensureConnected() {
    mutex.withLock {
      if (billingClient.connectionState == BillingClient.ConnectionState.CONNECTED) {
        return
      }

      suspendCancellableCoroutine { continuation ->
        billingClient.startConnection(
          object : BillingClientStateListener {
            override fun onBillingSetupFinished(result: BillingResult) {
              if (result.responseCode == BillingResponseCode.OK) {
                continuation.resume(Unit)
              } else {
                continuation.resumeWithException(
                  IllegalStateException(result.debugMessage.ifBlank { "Billing setup failed" })
                )
              }
            }

            override fun onBillingServiceDisconnected() = Unit
          }
        )
      }
    }
  }

  private fun List<ProductDetails>.toPurchaseProducts() = flatMap { product ->
    product.subscriptionOfferDetails.orEmpty().mapNotNull { offer ->
      val pricePhase = offer.pricingPhases.pricingPhaseList.lastOrNull() ?: return@mapNotNull null

      AndroidPurchaseProduct(
        productId = product.productId,
        planId = offer.basePlanId,
        name = product.title,
        price = pricePhase.formattedPrice,
        productDetails = product,
        offerToken = offer.offerToken,
      )
    }
  }

  private suspend fun handlePurchase(purchase: Purchase) {
    if (purchase.purchaseState != Purchase.PurchaseState.PURCHASED) {
      return
    }

    val productId = purchase.products.firstOrNull() ?: return

    _events.emit(
      PurchaseEvent(
        kind = PurchaseEventKind.Purchased,
        productId = productId,
        subscriptionId = purchase.purchaseToken,
        transactionId = purchase.orderId,
      )
    )
  }
}
