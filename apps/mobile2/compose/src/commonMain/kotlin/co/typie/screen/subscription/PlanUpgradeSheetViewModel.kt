package co.typie.screen.subscription

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.viewModelScope
import co.touchlab.kermit.Logger
import co.typie.graphql.EnrollPlanScreen_SubscribePlanWithTrial_Mutation
import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.PlanUpgradeSheet_Query
import co.typie.graphql.TypieError
import co.typie.graphql.type.buildUser
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.ui.state.AsyncAction
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class PlanUpgradeSheetViewModel(
  private val toast: Toast,
  private val subscriptionService: SubscriptionService,
  private val currentSubscriptionStore: CurrentSubscriptionStore,
) : GraphQLViewModel() {
  val query = watchQuery(
    placeholderData(),
    skip = { subscriptionService.usesSandbox },
  ) { PlanUpgradeSheet_Query() }

  val startTrialAction = AsyncAction(viewModelScope)

  var celebration by mutableStateOf<SubscriptionCelebration?>(null)
    private set

  fun startTrial() {
    startTrialAction.launch(
      onFailure = { e ->
        when (e) {
          is TypieError -> toast.show(ToastType.Error, e.message ?: DEFAULT_ERROR_MESSAGE)
          else -> {
            Logger.e(e) { "Failed to start subscription trial from upgrade sheet" }
            toast.show(ToastType.Error, DEFAULT_ERROR_MESSAGE)
          }
        }
      },
    ) {
        celebration = subscriptionService.startTrial {
          executeMutation(EnrollPlanScreen_SubscribePlanWithTrial_Mutation())
          currentSubscriptionStore.refresh()
          query.refetch()
        }
    }
  }
}

private fun placeholderData() = PlanUpgradeSheet_Query.Data(PlaceholderResolver) {
  me = buildUser {
    canStartTrial = false
  }
}

private const val DEFAULT_ERROR_MESSAGE = "오류가 발생했어요. 잠시 후 다시 시도해주세요."
