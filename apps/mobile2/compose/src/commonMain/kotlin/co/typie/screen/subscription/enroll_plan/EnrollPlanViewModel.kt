package co.typie.screen.subscription.enroll_plan

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.EnrollPlanScreen_Query
import co.typie.graphql.EnrollPlanScreen_SubscribeOrChangePlanWithInAppPurchase_Mutation
import co.typie.graphql.EnrollPlanScreen_SubscribePlanWithTrial_Mutation
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.TypieError
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildUser
import co.typie.graphql.executeMutation
import co.typie.graphql.type.InAppPurchaseStore
import co.typie.graphql.type.SubscribeOrChangePlanWithInAppPurchaseInput
import co.typie.graphql.watchQuery
import co.typie.platform.PurchaseEvent
import co.typie.platform.PurchasePlanInterval
import co.typie.platform.PurchaseProduct
import co.typie.platform.PurchaseStore
import co.typie.result.Result
import co.typie.graphql.Apollo
import co.typie.result.loading
import co.typie.result.result
import co.typie.service.CurrentSubscriptionStore
import co.typie.service.SubscriptionCelebration
import co.typie.service.SubscriptionService
import co.typie.service.purchaseCelebration
import co.typie.service.shouldShowPurchaseCelebration
import kotlinx.coroutines.launch

sealed interface EnrollPlanError {
  data object ServerError : EnrollPlanError
}

sealed interface PurchaseError {
  data class ServerError(val code: String?) : PurchaseError
  data object Unknown : PurchaseError
}

class EnrollPlanViewModel : ViewModel() {
  private val subscriptionService = SubscriptionService
  private val currentSubscriptionStore = CurrentSubscriptionStore
  var isStartingTrial by mutableStateOf(false)
    private set

  val query = Apollo.watchQuery(
    scope = viewModelScope,
    placeholderData = placeholderData(),
    skip = { subscriptionService.usesSandbox },
  ) { EnrollPlanScreen_Query() }

  var products by mutableStateOf<Map<PurchasePlanInterval, PurchaseProduct>>(emptyMap())
    private set

  var productsLoaded by mutableStateOf(false)
    private set

  var celebration by mutableStateOf<SubscriptionCelebration?>(null)
    private set

  var purchaseError by mutableStateOf<PurchaseError?>(null)
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

  suspend fun startTrial(): Result<Unit, EnrollPlanError> = loading({ isStartingTrial = it }) {
    try {
      celebration = subscriptionService.startTrial {
        Apollo.executeMutation(EnrollPlanScreen_SubscribePlanWithTrial_Mutation())
        currentSubscriptionStore.refresh()
        query.refetch()
      }
      // TODO: Mixpanel start_trial
    } catch (e: TypieError) {
      raise(EnrollPlanError.ServerError)
    }
  }

  suspend fun purchase(product: PurchaseProduct): Result<Unit, Nothing> = result {
    val purchaseResult = subscriptionService.purchase(
      product = product,
      accountId = query.data.me.uuid,
    )

    celebration = purchaseResult.celebration

    if (!purchaseResult.started) {
      throw IllegalStateException("Purchase not started")
    }
  }

  fun consumeCelebration() {
    celebration = null
  }

  fun consumePurchaseError() {
    purchaseError = null
  }

  private suspend fun loadProducts() {
    try {
      products = subscriptionService.queryProducts()
    } catch (_: Exception) {
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
      val response = Apollo.executeMutation(
        EnrollPlanScreen_SubscribeOrChangePlanWithInAppPurchase_Mutation(
          input = SubscribeOrChangePlanWithInAppPurchaseInput(
            data = verificationData,
            store = event.store.toGraphqlStore(),
          ),
        ),
      )

      query.refetch()
      currentSubscriptionStore.refresh()

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
      purchaseError = PurchaseError.ServerError(e.code)
    } catch (_: Exception) {
      purchaseError = PurchaseError.Unknown
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

