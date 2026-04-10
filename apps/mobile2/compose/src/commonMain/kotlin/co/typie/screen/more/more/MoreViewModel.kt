package co.typie.screen.more.more

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.MoreScreen_Query
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.text
import co.typie.graphql.type.buildUser
import co.typie.graphql.watchQuery
import com.apollographql.apollo.ApolloClient
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class MoreViewModel(
  private val apolloClient: ApolloClient,
) : ViewModel() {
  val query = apolloClient.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) { MoreScreen_Query() }
}

private fun placeholderData() = MoreScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    name = text(3..6)
    email = text(10..20)
    characterCountChanges = emptyList()
  }
}
