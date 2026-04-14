package co.typie.screen.subscription.cancelplan

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.domain.subscription.SubscriptionService
import co.typie.domain.subscription.SubscriptionServiceState
import co.typie.graphql.Apollo
import co.typie.graphql.CancelPlanScreen_Query
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildUser
import co.typie.graphql.type.SubscriptionState
import co.typie.graphql.watchQuery
import co.typie.platform.PlatformModule
import co.typie.result.Result
import co.typie.result.loading
import kotlinx.coroutines.launch

class CancelPlanViewModel : ViewModel() {
  val query =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) {
      CancelPlanScreen_Query()
    }

  var awaitingStoreResult by mutableStateOf(false)
    private set

  var shouldClose by mutableStateOf(false)
    private set

  var errorMessage by mutableStateOf<String?>(null)
    private set

  var isOpeningSubscriptionManagement by mutableStateOf(false)
    private set

  init {
    viewModelScope.launch {
      snapshotFlow { SubscriptionService.state }
        .collect { state ->
          if (!awaitingStoreResult) return@collect
          when (state) {
            is SubscriptionServiceState.Unknown -> return@collect
            is SubscriptionServiceState.NotSubscribed -> {}
            is SubscriptionServiceState.Subscribed ->
              if (state.subscription.state == SubscriptionState.ACTIVE) return@collect
          }
          awaitingStoreResult = false
          shouldClose = true
        }
    }
  }

  suspend fun openSubscriptionManagement(): Result<Unit, Nothing> =
    loading({ isOpeningSubscriptionManagement = it }) {
      val opened = PlatformModule.purchaseService.openSubscriptionManagement()
      if (opened) {
        awaitingStoreResult = true
      } else {
        errorMessage = "스토어를 열 수 없어요. 잠시 후 다시 시도해주세요."
      }
    }

  fun onResumed() {
    if (awaitingStoreResult) {
      SubscriptionService.refresh()
    }
  }

  fun consumeCloseRequest() {
    shouldClose = false
  }

  fun consumeErrorMessage() {
    errorMessage = null
  }
}

private fun placeholderData() =
  CancelPlanScreen_Query.Data(PlaceholderResolver) { me = buildUser {} }
