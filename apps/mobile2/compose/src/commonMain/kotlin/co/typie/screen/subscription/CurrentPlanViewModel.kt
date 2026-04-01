package co.typie.screen.subscription

import co.typie.graphql.CurrentPlanScreen_Query
import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.type.buildUser
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class CurrentPlanViewModel(
  subscriptionService: SubscriptionService,
) : GraphQLViewModel() {
  val query = watchQuery(
    placeholderData(),
    skip = { subscriptionService.usesSandbox },
  ) { CurrentPlanScreen_Query() }
}

private fun placeholderData() = CurrentPlanScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    credit = 0
    subscription = null
  }
}
