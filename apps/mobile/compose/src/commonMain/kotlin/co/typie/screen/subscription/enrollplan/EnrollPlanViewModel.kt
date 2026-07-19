package co.typie.screen.subscription.enrollplan

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.domain.subscription.SubscriptionService
import co.typie.graphql.Apollo
import co.typie.graphql.EnrollPlanScreen_Query
import co.typie.graphql.EnrollPlanScreen_SubscribePlanWithTrial_Mutation
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.TypieError
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildUser
import co.typie.graphql.executeMutation
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.loading

internal sealed interface EnrollPlanError {
  data object SubscriptionHistoryExists : EnrollPlanError

  data object TrialAlreadyUsed : EnrollPlanError
}

internal class EnrollPlanViewModel : ViewModel() {
  val query =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) {
      EnrollPlanScreen_Query()
    }

  var isEnrollingTrial by mutableStateOf(false)
    private set

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
}

private fun placeholderData() =
  EnrollPlanScreen_Query.Data(PlaceholderResolver) { me = buildUser { canStartTrial = false } }
