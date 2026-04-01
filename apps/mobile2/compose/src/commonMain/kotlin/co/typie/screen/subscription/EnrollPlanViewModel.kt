package co.typie.screen.subscription

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.viewModelScope
import co.touchlab.kermit.Logger
import co.typie.graphql.EnrollPlanScreen_Query
import co.typie.graphql.EnrollPlanScreen_SubscribeOrChangePlanWithInAppPurchase_Mutation
import co.typie.graphql.EnrollPlanScreen_SubscribePlanWithTrial_Mutation
import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.TypieError
import co.typie.graphql.type.InAppPurchaseStore
import co.typie.graphql.type.SubscribeOrChangePlanWithInAppPurchaseInput
import co.typie.graphql.type.buildUser
import co.typie.overlay.Loader
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.platform.PurchaseEvent
import co.typie.platform.PurchasePlanInterval
import co.typie.platform.PurchaseProduct
import co.typie.platform.PurchaseStore
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class EnrollPlanViewModel(
  private val loader: Loader,
  private val toast: Toast,
  private val subscriptionService: SubscriptionService,
) : GraphQLViewModel() {
  private val purchaseStartMutex = Mutex()

  val query = watchQuery(
    placeholderData(),
    skip = { subscriptionService.usesSandbox },
  ) { EnrollPlanScreen_Query() }

  var products by mutableStateOf<Map<PurchasePlanInterval, PurchaseProduct>>(emptyMap())
    private set

  var productsLoaded by mutableStateOf(false)
    private set

  var celebration by mutableStateOf<SubscriptionCelebration?>(null)
    private set

  init {
    viewModelScope.launch {
      loadProducts()
    }

    viewModelScope.launch {
      subscriptionService.purchaseEvents.collect { event ->
        handlePurchaseEvent(event)
      }
    }
  }

  suspend fun startTrial() {
    try {
      celebration = loader.runWith {
        subscriptionService.startTrial {
          executeMutation(EnrollPlanScreen_SubscribePlanWithTrial_Mutation())
          query.refetch()
        }
      }
      // TODO: Mixpanel start_trial
    } catch (e: TypieError) {
      toast.show(ToastType.Error, e.message ?: DEFAULT_ERROR_MESSAGE)
    } catch (e: Exception) {
      Logger.e(e) { "Failed to start subscription trial" }
      toast.show(ToastType.Error, DEFAULT_ERROR_MESSAGE)
    }
  }

  suspend fun purchase(product: PurchaseProduct) {
    if (!purchaseStartMutex.tryLock()) {
      return
    }

    try {
      val result = loader.runWith {
        subscriptionService.purchase(
          product = product,
          accountId = query.data.me.uuid,
        )
      }

      celebration = result.celebration

      if (!result.started) {
        toast.show(ToastType.Error, PURCHASE_START_FAILURE_MESSAGE)
      }
    } catch (e: Exception) {
      Logger.e(e) { "Failed to start subscription purchase" }
      toast.show(ToastType.Error, PURCHASE_START_FAILURE_MESSAGE)
    } finally {
      purchaseStartMutex.unlock()
    }
  }

  fun consumeCelebration() {
    celebration = null
  }

  private suspend fun loadProducts() {
    try {
      products = subscriptionService.queryProducts()
    } catch (error: Exception) {
      Logger.e(error) { "Failed to query subscription products" }
      products = emptyMap()
    } finally {
      productsLoaded = true
    }
  }

  private suspend fun handlePurchaseEvent(event: PurchaseEvent) {
    val originalSubscriptionId = query.data.me.subscription?.id
    val originalPlanId = query.data.me.subscription?.plan?.id
    val verificationData = event.verificationData()

    try {
      val response = executeMutation(
        EnrollPlanScreen_SubscribeOrChangePlanWithInAppPurchase_Mutation(
          input = SubscribeOrChangePlanWithInAppPurchaseInput(
            data = verificationData,
            store = event.store.toGraphqlStore(),
          ),
        ),
      )

      query.refetch()
      subscriptionService.notifyChanged()

      if (
        shouldShowPurchaseCelebration(
          originalSubscriptionId = originalSubscriptionId,
          originalPlanId = originalPlanId,
          updatedSubscriptionId = response.subscribeOrChangePlanWithInAppPurchase.id,
          updatedPlanId = response.subscribeOrChangePlanWithInAppPurchase.plan.id,
        )
      ) {
        // TODO: Mixpanel enroll_plan / Appsflyer complete_subscription
        celebration = purchaseCelebration()
      }
    } catch (e: TypieError) {
      toast.show(ToastType.Error, e.message ?: DEFAULT_ERROR_MESSAGE)
    } catch (e: Exception) {
      Logger.e(e) { "Failed to verify in-app purchase" }
      toast.show(ToastType.Error, DEFAULT_ERROR_MESSAGE)
    }
  }
}

private fun PurchaseEvent.verificationData(): String {
  return when (this) {
    is PurchaseEvent.Purchased -> verificationData
    is PurchaseEvent.Restored -> verificationData
  }
}

private fun PurchaseStore.toGraphqlStore(): InAppPurchaseStore {
  return when (this) {
    PurchaseStore.AppStore -> InAppPurchaseStore.APP_STORE
    PurchaseStore.GooglePlay -> InAppPurchaseStore.GOOGLE_PLAY
  }
}

private fun placeholderData() = EnrollPlanScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    uuid = ""
    canStartTrial = false
    subscription = null
  }
}

private const val DEFAULT_ERROR_MESSAGE = "오류가 발생했어요. 잠시 후 다시 시도해주세요."
private const val PURCHASE_START_FAILURE_MESSAGE = "결제를 시작할 수 없어요. 잠시 후 다시 시도해주세요."
