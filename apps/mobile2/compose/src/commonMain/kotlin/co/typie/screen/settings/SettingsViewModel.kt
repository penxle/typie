package co.typie.screen.settings

import androidx.lifecycle.viewModelScope
import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.SettingsScreen_Query
import co.typie.graphql.type.buildUser
import co.typie.screen.subscription.SubscriptionSync
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.launch
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class SettingsViewModel(
  private val subscriptionSync: SubscriptionSync,
) : GraphQLViewModel() {
  val query = watchQuery(placeholderData()) { SettingsScreen_Query() }

  init {
    viewModelScope.launch {
      subscriptionSync.events.collect {
        query.refetch()
      }
    }
  }
}

private fun placeholderData() = SettingsScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    subscription = null
  }
}
