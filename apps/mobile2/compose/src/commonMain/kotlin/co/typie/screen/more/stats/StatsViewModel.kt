package co.typie.screen.more.stats

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.StatsScreen_Query
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildUser
import co.typie.graphql.builder.buildUserUsage
import co.typie.graphql.text
import co.typie.graphql.Apollo
import co.typie.graphql.watchQuery

class StatsViewModel : ViewModel() {

  val query = Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) { StatsScreen_Query() }
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
