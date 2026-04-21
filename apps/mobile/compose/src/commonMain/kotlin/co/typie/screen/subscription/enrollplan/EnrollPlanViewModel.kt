package co.typie.screen.subscription.enrollplan

import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.domain.subscription.SubscriptionService
import co.typie.graphql.Apollo
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
import co.typie.platform.ActivityContext
import co.typie.platform.Platform
import co.typie.platform.PlatformModule
import co.typie.platform.PurchaseEvent
import co.typie.platform.PurchaseProduct
import co.typie.result.Result
import co.typie.result.loading
import co.typie.result.result
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.launch

internal sealed interface EnrollPlanEvent {
  data object PurchaseCompleted : EnrollPlanEvent
}

internal sealed interface EnrollPlanError {
  data object SubscriptionHistoryExists : EnrollPlanError

  data object TrialAlreadyUsed : EnrollPlanError
}

internal class EnrollPlanViewModel : ViewModel() {
  private val _events = Channel<EnrollPlanEvent>(Channel.BUFFERED)
  val events = _events.receiveAsFlow()

  val query =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) {
      EnrollPlanScreen_Query()
    }

  var isEnrollingTrial by mutableStateOf(false)
    private set

  var products by mutableStateOf<List<PurchaseProduct>>(emptyList())
    private set

  val monthlyProduct by derivedStateOf { products.firstOrNull { it.planId == "pl0fl1map" } }
  val yearlyProduct by derivedStateOf { products.firstOrNull { it.planId == "pl0fl1yap" } }

  init {
    viewModelScope.launch {
      launch { PlatformModule.purchaseService.events.collect { handlePurchaseEvent(it) } }

      when (PlatformModule.platform) {
        Platform.iOS -> {
          products = PlatformModule.purchaseService.queryProducts(listOf("pl0fl1map", "pl0fl1yap"))
        }
        Platform.Android -> {
          products = PlatformModule.purchaseService.queryProducts(listOf("plan.full"))
        }
        else -> {}
      }
    }
  }

  suspend fun enrollTrial(): Result<Unit, EnrollPlanError> =
    loading({ isEnrollingTrial = it }) {
      try {
        Apollo.executeMutation(EnrollPlanScreen_SubscribePlanWithTrial_Mutation())
        SubscriptionService.refresh()
      } catch (e: TypieError) {
        when (e.code) {
          "subscription_history_exists" -> raise(EnrollPlanError.SubscriptionHistoryExists)
          "trial_already_used" -> raise(EnrollPlanError.TrialAlreadyUsed)
          else -> throw e
        }
      }
    }

  context(_: ActivityContext)
  suspend fun purchase(product: PurchaseProduct): Result<Unit, Nothing> = result {
    PlatformModule.purchaseService.purchase(product = product, accountId = query.data.me.uuid)
  }

  private suspend fun handlePurchaseEvent(event: PurchaseEvent) {
    val originalSubscriptionId = query.data.me.subscription?.id

    try {
      val response =
        Apollo.executeMutation(
          EnrollPlanScreen_SubscribeOrChangePlanWithInAppPurchase_Mutation(
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

      if (originalSubscriptionId != response.subscribeOrChangePlanWithInAppPurchase.id) {
        _events.send(EnrollPlanEvent.PurchaseCompleted)
      }
    } catch (e: CancellationException) {
      throw e
    } catch (_: Exception) {
      // best effort
    }
  }
}

private fun placeholderData() =
  EnrollPlanScreen_Query.Data(PlaceholderResolver) {
    me = buildUser {
      canStartTrial = false
      subscription = null
    }
  }
