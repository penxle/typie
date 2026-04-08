package co.typie.screen.settings

import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.SettingsScreen_Query
import co.typie.graphql.type.buildUser
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class SettingsViewModel : GraphQLViewModel() {
  val query = watchQuery(placeholderData()) { SettingsScreen_Query() }
}

private fun placeholderData() = SettingsScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    subscription = null
  }
}
