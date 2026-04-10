package co.typie.screen.more.more

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.Apollo
import co.typie.graphql.MoreScreen_Query
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildUser
import co.typie.graphql.text
import co.typie.graphql.watchQuery

class MoreViewModel : ViewModel() {

  val query =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) {
      MoreScreen_Query()
    }
}

private fun placeholderData() =
  MoreScreen_Query.Data(PlaceholderResolver) {
    me = buildUser {
      name = text(3..6)
      email = text(10..20)
      characterCountChanges = emptyList()
    }
  }
