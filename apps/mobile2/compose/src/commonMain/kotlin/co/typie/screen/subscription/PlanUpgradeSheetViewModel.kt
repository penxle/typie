package co.typie.screen.subscription

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.EnrollPlanScreen_SubscribePlanWithTrial_Mutation
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.PlanUpgradeSheet_Query
import co.typie.graphql.TypieError
import co.typie.graphql.executeMutation
import co.typie.graphql.type.buildUser
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.loading
import com.apollographql.apollo.ApolloClient
import org.koin.core.annotation.KoinViewModel

sealed interface PlanUpgradeTrialError {
  data object ServerError : PlanUpgradeTrialError
}

@KoinViewModel
class PlanUpgradeSheetViewModel(
  private val apolloClient: ApolloClient,
  private val subscriptionService: SubscriptionService,
  private val currentSubscriptionStore: CurrentSubscriptionStore,
) : ViewModel() {
  val query = apolloClient.watchQuery(
    scope = viewModelScope,
    placeholderData = placeholderData(),
    skip = { subscriptionService.usesSandbox },
  ) { PlanUpgradeSheet_Query() }

  var isStartingTrial by mutableStateOf(false)
    private set

  var celebration by mutableStateOf<SubscriptionCelebration?>(null)
    private set

  suspend fun startTrial(): Result<Unit, PlanUpgradeTrialError> {
    return loading({ isStartingTrial = it }) {
      try {
        celebration = subscriptionService.startTrial {
          apolloClient.executeMutation(EnrollPlanScreen_SubscribePlanWithTrial_Mutation())
          currentSubscriptionStore.refresh()
          query.refetch()
        }
      } catch (e: TypieError) {
        raise(PlanUpgradeTrialError.ServerError)
      }
    }
  }
}

private fun placeholderData() = PlanUpgradeSheet_Query.Data(PlaceholderResolver) {
  me = buildUser {
    canStartTrial = false
  }
}

