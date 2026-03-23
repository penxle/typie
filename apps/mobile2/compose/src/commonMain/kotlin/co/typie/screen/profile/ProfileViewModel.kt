package co.typie.screen.profile

import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.ProfileScreen_Query
import co.typie.graphql.text
import co.typie.graphql.type.buildUser
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class ProfileViewModel : GraphQLViewModel() {
  val query = watchQuery(placeholderData()) { ProfileScreen_Query() }
}

private fun placeholderData() = ProfileScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    name = text(3..6)
    email = text(10..20)
    characterCountChanges = emptyList()
  }
}
