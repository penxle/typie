package co.typie.screen.subscription.cancel_plan

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.QueryState
import co.typie.result.Result
import co.typie.service.CancelPlanFlowState
import co.typie.service.CurrentSubscriptionStore
import co.typie.service.SubscriptionManagementResult
import co.typie.service.SubscriptionService
import co.typie.service.consumeCancelPlanCloseRequest
import co.typie.service.consumeCancelPlanErrorMessage
import co.typie.service.reduceCancelPlanFlowOnManagementResult
import co.typie.service.reduceCancelPlanFlowOnSubscriptionState
import co.typie.result.loading
import co.typie.result.result
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.launch
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class CancelPlanViewModel(
  private val currentSubscriptionStore: CurrentSubscriptionStore,
  private val subscriptionService: SubscriptionService,
) : ViewModel() {
  var flowState by mutableStateOf(CancelPlanFlowState())
    private set

  var isOpeningSubscriptionManagement by mutableStateOf(false)
    private set

  val shouldClose: Boolean
    get() = flowState.shouldClose

  val errorMessage: String?
    get() = flowState.errorMessage

  init {
    viewModelScope.launch {
      currentSubscriptionStore.state.collect { state ->
        val subscriptionState = (state as? QueryState.Success)?.data?.state
        updateFlowState(
          reduceCancelPlanFlowOnSubscriptionState(
            current = flowState,
            subscriptionState = subscriptionState,
          ),
        )
      }
    }
  }

  fun onOpenSubscriptionManagementResult(
    result: SubscriptionManagementResult,
  ) {
    updateFlowState(
      reduceCancelPlanFlowOnManagementResult(
        current = flowState,
        result = result,
      ),
    )
  }

  suspend fun openSubscriptionManagement(): Result<Unit, Nothing> = loading({ isOpeningSubscriptionManagement = it }) {
    onOpenSubscriptionManagementResult(subscriptionService.openSubscriptionManagement())
  }

  fun onResumed() {
    if (flowState.awaitingStoreResult) {
      currentSubscriptionStore.refresh()
    }
  }

  fun consumeCloseRequest() {
    flowState = consumeCancelPlanCloseRequest(flowState)
  }

  fun consumeErrorMessage() {
    flowState = consumeCancelPlanErrorMessage(flowState)
  }

  private fun updateFlowState(next: CancelPlanFlowState) {
    flowState = next
  }
}
