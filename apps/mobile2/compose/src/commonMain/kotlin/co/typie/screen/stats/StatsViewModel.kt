package co.typie.screen.stats

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.StatsScreen_Query
import co.typie.graphql.text
import co.typie.graphql.type.buildUser
import co.typie.graphql.type.buildUserUsage
import co.typie.graphql.watchQuery
import com.apollographql.apollo.ApolloClient
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class StatsViewModel(
  private val apolloClient: ApolloClient,
) : ViewModel() {
  val query = apolloClient.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) { StatsScreen_Query() }
}

private fun placeholderData() = StatsScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    name = text(3..6)
    documentCount = 0
    usage = buildUserUsage {
      totalCharacterCount = 0
    }
    characterCountChanges = emptyList()
  }
}
