package co.typie.platform

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.net.Uri
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
import com.android.billingclient.api.PurchasesUpdatedListener
import com.android.billingclient.api.QueryProductDetailsParams
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import co.typie.screen.subscription.FULL_ACCESS_GOOGLE_PLAY_PRODUCT_ID
import co.typie.screen.subscription.FULL_ACCESS_MONTHLY_STORE_PRODUCT_ID
import co.typie.screen.subscription.FULL_ACCESS_YEARLY_STORE_PRODUCT_ID
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

internal object PurchaseActivityHolder {
  private var activity: Activity? = null

  fun attach(activity: Activity) {
    this.activity = activity
  }

  fun detach(activity: Activity) {
    if (this.activity === activity) {
      this.activity = null
    }
  }

  fun current(): Activity? = activity
}

private data class AndroidAvailablePurchase(
  val productDetails: ProductDetails,
  val offerToken: String,
  val product: PurchaseProduct,
)

internal class AndroidPurchaseService(
  private val context: Context,
) : PurchaseService {
  private val serviceScope = CoroutineScope(SupervisorJob() + Dispatchers.Main)
  private val connectionMutex = Mutex()
  private val productCache = mutableMapOf<String, AndroidAvailablePurchase>()
  private var pendingProductId: String? = null

  private val billingClient = BillingClient.newBuilder(context)
    .setListener(PurchaseUpdatesListener())
    .enablePendingPurchases(
      PendingPurchasesParams.newBuilder()
        .enableOneTimeProducts()
        .build(),
    )
    .build()

  override val events: SharedFlow<PurchaseEvent> = MutableSharedFlow(
    replay = 0,
    extraBufferCapacity = 8,
  )

  private val mutableEvents: MutableSharedFlow<PurchaseEvent>
    get() = events as MutableSharedFlow<PurchaseEvent>

  override suspend fun queryProducts(): Map<PurchasePlanInterval, PurchaseProduct> {
    ensureConnected()

    val params = QueryProductDetailsParams.newBuilder()
      .setProductList(
        listOf(
          QueryProductDetailsParams.Product.newBuilder()
            .setProductId(FULL_ACCESS_GOOGLE_PLAY_PRODUCT_ID)
            .setProductType(ProductType.SUBS)
            .build(),
        ),
      )
      .build()

    return suspendCancellableCoroutine { continuation ->
      billingClient.queryProductDetailsAsync(params) { billingResult, queryResult ->
        if (billingResult.responseCode != BillingResponseCode.OK) {
          continuation.resumeWithException(
            IllegalStateException(billingResult.debugMessage.ifBlank { "Failed to query Google Play subscription products" }),
          )
          return@queryProductDetailsAsync
        }

        val mapped = buildAndroidPurchaseProducts(queryResult.productDetailsList.orEmpty())
        productCache.clear()
        productCache.putAll(mapped.associateBy { it.product.id })
        continuation.resume(mapped.associateBy { it.product.interval }.mapValues { (_, value) -> value.product })
      }
    }
  }

  override suspend fun purchase(
    product: PurchaseProduct,
    accountId: String,
  ): Boolean {
    ensureConnected()

    val activity = PurchaseActivityHolder.current() ?: return false
    val availableProduct = productCache[product.id] ?: queryProducts()[product.interval]?.let { productCache[product.id] }
      ?: return false

    pendingProductId = availableProduct.product.id

    val flowParams = BillingFlowParams.newBuilder()
      .setProductDetailsParamsList(
        listOf(
          BillingFlowParams.ProductDetailsParams.newBuilder()
            .setProductDetails(availableProduct.productDetails)
            .setOfferToken(availableProduct.offerToken)
            .build(),
        ),
      )
      .setObfuscatedAccountId(accountId)
      .build()

    val result = billingClient.launchBillingFlow(activity, flowParams)
    if (result.responseCode != BillingResponseCode.OK) {
      pendingProductId = null
    }
    return result.responseCode == BillingResponseCode.OK
  }

  override suspend fun openSubscriptionManagement(): Boolean {
    val intent = Intent(
      Intent.ACTION_VIEW,
      Uri.parse(
        "https://play.google.com/store/account/subscriptions?package=${context.packageName}&sku=$FULL_ACCESS_GOOGLE_PLAY_PRODUCT_ID",
      ),
    ).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)

    return runCatching {
      context.startActivity(intent)
      true
    }.getOrDefault(false)
  }

  private suspend fun ensureConnected() {
    connectionMutex.withLock {
      if (billingClient.connectionState == BillingClient.ConnectionState.CONNECTED) {
        return
      }

      suspendCancellableCoroutine { continuation ->
        billingClient.startConnection(
          object : BillingClientStateListener {
            override fun onBillingServiceDisconnected() = Unit

            override fun onBillingSetupFinished(billingResult: BillingResult) {
              if (billingResult.responseCode == BillingResponseCode.OK) {
                continuation.resume(Unit)
              } else {
                continuation.resumeWithException(
                  IllegalStateException(billingResult.debugMessage.ifBlank { "Billing setup failed" }),
                )
              }
            }
          },
        )
      }
    }
  }

  private fun buildAndroidPurchaseProducts(
    productDetailsList: List<ProductDetails>,
  ): List<AndroidAvailablePurchase> {
    val byBasePlanId = linkedMapOf<String, AndroidAvailablePurchase>()

    productDetailsList.forEach { productDetails ->
      productDetails.subscriptionOfferDetails.orEmpty().forEach { offerDetails ->
        val interval = when (offerDetails.basePlanId) {
          FULL_ACCESS_MONTHLY_STORE_PRODUCT_ID -> PurchasePlanInterval.Monthly
          FULL_ACCESS_YEARLY_STORE_PRODUCT_ID -> PurchasePlanInterval.Yearly
          else -> null
        } ?: return@forEach

        val pricePhase = offerDetails.pricingPhases.pricingPhaseList.lastOrNull() ?: return@forEach

        byBasePlanId[offerDetails.basePlanId] = AndroidAvailablePurchase(
          productDetails = productDetails,
          offerToken = offerDetails.offerToken,
          product = PurchaseProduct(
            id = offerDetails.basePlanId,
            interval = interval,
            price = pricePhase.formattedPrice,
            title = productDetails.title,
            rawPrice = pricePhase.priceAmountMicros / 1_000_000.0,
            currencyCode = pricePhase.priceCurrencyCode,
          ),
        )
      }
    }

    return byBasePlanId.values.toList()
  }

  private suspend fun handlePurchase(purchase: Purchase) {
    if (purchase.purchaseState != Purchase.PurchaseState.PURCHASED) {
      return
    }

    val productId = pendingProductId ?: purchase.products.firstOrNull() ?: FULL_ACCESS_GOOGLE_PLAY_PRODUCT_ID

    mutableEvents.emit(
      PurchaseEvent.Purchased(
        productId = productId,
        store = PurchaseStore.GooglePlay,
        verificationData = purchase.purchaseToken,
        purchaseId = purchase.orderId,
      ),
    )

    if (!purchase.isAcknowledged) {
      val params = AcknowledgePurchaseParams.newBuilder()
        .setPurchaseToken(purchase.purchaseToken)
        .build()

      suspendCancellableCoroutine { continuation ->
        billingClient.acknowledgePurchase(params) {
          continuation.resume(Unit)
        }
      }
    }

    pendingProductId = null
  }

  private inner class PurchaseUpdatesListener : PurchasesUpdatedListener {
    override fun onPurchasesUpdated(
      billingResult: BillingResult,
      purchases: MutableList<Purchase>?,
    ) {
      if (billingResult.responseCode != BillingResponseCode.OK || purchases == null) {
        if (billingResult.responseCode != BillingResponseCode.USER_CANCELED) {
          pendingProductId = null
        }
        return
      }

      purchases.forEach { purchase ->
        serviceScope.launch {
          handlePurchase(purchase)
        }
      }
    }
  }
}
