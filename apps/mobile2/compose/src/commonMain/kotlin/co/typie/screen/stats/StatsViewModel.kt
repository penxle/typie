package co.typie.screen.stats

import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.StatsScreen_Query
import co.typie.graphql.text
import co.typie.graphql.type.buildUser
import co.typie.graphql.type.buildUserUsage
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class StatsViewModel : GraphQLViewModel() {
  val query = watchQuery(placeholderData()) { StatsScreen_Query() }
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
