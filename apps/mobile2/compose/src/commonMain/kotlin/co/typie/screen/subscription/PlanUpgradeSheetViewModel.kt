package co.typie.screen.subscription

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.touchlab.kermit.Logger
import co.typie.graphql.EnrollPlanScreen_SubscribePlanWithTrial_Mutation
import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.PlanUpgradeSheet_Query
import co.typie.graphql.TypieError
import co.typie.graphql.type.buildUser
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class PlanUpgradeSheetViewModel(
  private val toast: Toast,
  private val subscriptionService: SubscriptionService,
) : GraphQLViewModel() {
  val query = watchQuery(
    placeholderData(),
    skip = { subscriptionService.usesSandbox },
  ) { PlanUpgradeSheet_Query() }

  var isStartingTrial by mutableStateOf(false)
    private set

  var celebration by mutableStateOf<SubscriptionCelebration?>(null)
    private set

  suspend fun startTrial() {
    if (isStartingTrial) return

    isStartingTrial = true
    try {
      celebration = subscriptionService.startTrial {
        executeMutation(EnrollPlanScreen_SubscribePlanWithTrial_Mutation())
        query.refetch()
      }
    } catch (e: TypieError) {
      toast.show(ToastType.Error, e.message ?: DEFAULT_ERROR_MESSAGE)
    } catch (e: Exception) {
      // TODO: sentry
      toast.show(ToastType.Error, DEFAULT_ERROR_MESSAGE)
    } finally {
      isStartingTrial = false
    }
  }
}

private fun placeholderData() = PlanUpgradeSheet_Query.Data(PlaceholderResolver) {
  me = buildUser {
    canStartTrial = false
  }
}

private const val DEFAULT_ERROR_MESSAGE = "오류가 발생했어요. 잠시 후 다시 시도해주세요."
