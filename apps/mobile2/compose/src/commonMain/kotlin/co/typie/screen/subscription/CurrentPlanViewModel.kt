package co.typie.screen.subscription

import androidx.lifecycle.viewModelScope
import co.typie.graphql.CurrentPlanScreen_Query
import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.type.buildUser
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.launch
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class CurrentPlanViewModel(
  subscriptionService: SubscriptionService,
  private val subscriptionSync: SubscriptionSync,
) : GraphQLViewModel() {
  val query = watchQuery(
    placeholderData(),
    skip = { subscriptionService.usesSandbox },
  ) { CurrentPlanScreen_Query() }

  init {
    viewModelScope.launch {
      subscriptionSync.events.collect {
        query.refetch()
      }
    }
  }
}

private fun placeholderData() = CurrentPlanScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    credit = 0
    subscription = null
  }
}
