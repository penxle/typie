package co.typie.domain.subscription

import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import co.typie.graphql.Apollo
import co.typie.graphql.SubscriptionPurchaseService_Query
import co.typie.graphql.SubscriptionPurchaseService_SubscribeOrChangePlanWithInAppPurchase_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.type.InAppPurchaseStore
import co.typie.graphql.type.SubscribeOrChangePlanWithInAppPurchaseInput
import co.typie.platform.ActivityContext
import co.typie.platform.Platform
import co.typie.platform.PlatformModule
import co.typie.platform.PurchaseEvent
import co.typie.platform.PurchaseProduct
import kotlin.coroutines.cancellation.CancellationException
import kotlin.time.Duration.Companion.seconds
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withTimeoutOrNull

object SubscriptionPurchaseService {
  private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
  private val productsMutex = Mutex()

  var products by mutableStateOf<List<PurchaseProduct>>(emptyList())
    private set

  var productsUnavailable by mutableStateOf(false)
    private set

  val monthlyProduct: PurchaseProduct? by derivedStateOf {
    products.firstOrNull { it.planId == "pl0fl1map" }
  }

  val yearlyProduct: PurchaseProduct? by derivedStateOf {
    products.firstOrNull { it.planId == "pl0fl1yap" }
  }

  private val _completions = MutableSharedFlow<Unit>()
  val completions: SharedFlow<Unit> = _completions

  var registrationGeneration by mutableStateOf(0L)
    private set

  private var launched = false

  fun launch() {
    if (launched) return
    launched = true

    scope.launch(start = CoroutineStart.UNDISPATCHED) {
      PlatformModule.purchaseService.events.collect { handlePurchaseEvent(it) }
    }
  }

  suspend fun ensureProductsLoaded() {
    productsMutex.withLock {
      if (products.isNotEmpty()) {
        return
      }

      try {
        products =
          PlatformModule.purchaseService.queryProducts(storeProductIds(PlatformModule.platform))
        productsUnavailable = products.isEmpty()
      } catch (e: CancellationException) {
        throw e
      } catch (_: Exception) {
        productsUnavailable = true
      }
    }
  }

  context(_: ActivityContext)
  suspend fun purchase(product: PurchaseProduct): Boolean {
    val accountId =
      try {
        Apollo.query(SubscriptionPurchaseService_Query()).execute().dataOrThrow().me.uuid
      } catch (e: CancellationException) {
        throw e
      } catch (_: Exception) {
        return false
      }

    return PlatformModule.purchaseService.purchase(product = product, accountId = accountId)
  }

  suspend fun awaitRegistration(sinceGeneration: Long) {
    withTimeoutOrNull(15.seconds) {
      snapshotFlow { registrationGeneration }.first { it > sinceGeneration }
    }
  }

  private suspend fun handlePurchaseEvent(event: PurchaseEvent) {
    try {
      val previousSubscriptionId = SubscriptionService.subscription?.id

      val response =
        Apollo.executeMutation(
          SubscriptionPurchaseService_SubscribeOrChangePlanWithInAppPurchase_Mutation(
            input =
              SubscribeOrChangePlanWithInAppPurchaseInput(
                data = event.subscriptionId,
                store =
                  when (PlatformModule.platform) {
                    Platform.Android -> InAppPurchaseStore.GOOGLE_PLAY
                    Platform.iOS -> InAppPurchaseStore.APP_STORE
                    else ->
                      throw IllegalArgumentException(
                        "Unsupported platform: ${PlatformModule.platform}"
                      )
                  },
              )
          )
        )

      PlatformModule.purchaseService.finishTransaction(event.subscriptionId)
      SubscriptionService.refresh()

      if (
        isNewSubscription(
          previousSubscriptionId,
          response.subscribeOrChangePlanWithInAppPurchase.id,
        )
      ) {
        _completions.emit(Unit)
      }
    } catch (e: CancellationException) {
      throw e
    } catch (_: Exception) {
      // best effort
    } finally {
      registrationGeneration += 1
    }
  }
}

internal fun storeProductIds(platform: Platform): List<String> =
  when (platform) {
    Platform.Android -> listOf("plan.full")
    else -> listOf("pl0fl1map", "pl0fl1yap")
  }

internal fun isNewSubscription(
  previousSubscriptionId: String?,
  newSubscriptionId: String,
): Boolean = previousSubscriptionId != newSubscriptionId
