package co.typie.screen.subscription

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.lifecycle.viewModelScope
import co.typie.graphql.CancelPlanScreen_Query
import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.QueryState
import co.typie.graphql.type.buildUser
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.launch
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class CancelPlanViewModel(
  private val subscriptionService: SubscriptionService,
) : GraphQLViewModel() {
  val query = watchQuery(
    placeholderData(),
    skip = { subscriptionService.usesSandbox },
  ) { CancelPlanScreen_Query() }

  var flowState by mutableStateOf(CancelPlanFlowState())
    private set

  val shouldClose: Boolean
    get() = flowState.shouldClose

  val errorMessage: String?
    get() = flowState.errorMessage

  init {
    viewModelScope.launch {
      snapshotFlow { (query.state as? QueryState.Success)?.data?.me?.subscription?.state }
        .distinctUntilChanged()
        .collect { subscriptionState ->
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

  fun onResumed() {
    if (flowState.awaitingStoreResult) {
      query.refetch()
    }
  }

  fun consumeCloseRequest() {
    flowState = consumeCancelPlanCloseRequest(flowState)
  }

  fun consumeErrorMessage() {
    flowState = consumeCancelPlanErrorMessage(flowState)
  }

  private fun updateFlowState(next: CancelPlanFlowState) {
    if (!flowState.shouldClose && next.shouldClose) {
      subscriptionService.notifyChanged()
    }

    flowState = next
  }
}

private fun placeholderData() = CancelPlanScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    subscription = null
  }
}
