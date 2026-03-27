package co.typie.screen.security_settings

import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.SecuritySettingsScreen_Query
import co.typie.graphql.type.buildUser
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class SecuritySettingsViewModel : GraphQLViewModel() {
  val query = watchQuery(
    placeholderData = placeholderData(),
  ) { SecuritySettingsScreen_Query() }
}

private fun placeholderData() = SecuritySettingsScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    hasPassword = true
  }
}
