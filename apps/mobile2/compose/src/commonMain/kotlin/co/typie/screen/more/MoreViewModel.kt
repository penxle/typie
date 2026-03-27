package co.typie.screen.more

import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.MoreScreen_Query
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.text
import co.typie.graphql.type.buildUser
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class MoreViewModel : GraphQLViewModel() {
  val query = watchQuery(placeholderData()) { MoreScreen_Query() }
}

private fun placeholderData() = MoreScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    name = text(3..6)
    email = text(10..20)
    characterCountChanges = emptyList()
  }
}
