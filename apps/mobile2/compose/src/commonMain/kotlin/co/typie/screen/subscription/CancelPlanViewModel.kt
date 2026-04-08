package co.typie.screen.subscription

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.touchlab.kermit.Logger
import co.typie.graphql.QueryState
import co.typie.ui.state.AsyncAction
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

  val openSubscriptionManagementAction = AsyncAction(viewModelScope)

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

  fun openSubscriptionManagement() {
    openSubscriptionManagementAction.launch(
      onFailure = { e ->
        Logger.e(e) { "Failed to open subscription management" }
        onOpenSubscriptionManagementResult(SubscriptionManagementResult.FailedToOpen)
      },
    ) {
        onOpenSubscriptionManagementResult(subscriptionService.openSubscriptionManagement())
    }
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
