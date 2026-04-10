package co.typie.screen.settings.security_settings

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.SecuritySettingsScreen_Query
import co.typie.graphql.type.buildUser
import co.typie.graphql.watchQuery
import com.apollographql.apollo.ApolloClient
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class SecuritySettingsViewModel(
  private val apolloClient: ApolloClient,
) : ViewModel() {
  val query = apolloClient.watchQuery(
    scope = viewModelScope,
    placeholderData = placeholderData(),
  ) { SecuritySettingsScreen_Query() }
}

private fun placeholderData() = SecuritySettingsScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    hasPassword = true
  }
}
