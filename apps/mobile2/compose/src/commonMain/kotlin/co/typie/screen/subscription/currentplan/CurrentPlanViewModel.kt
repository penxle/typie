package co.typie.screen.subscription.currentplan

import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.Apollo
import co.typie.graphql.CurrentPlanScreen_Query
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildUser
import co.typie.graphql.watchQuery

class CurrentPlanViewModel : ViewModel() {
  val query =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) {
      CurrentPlanScreen_Query()
    }
}

private fun placeholderData() =
  CurrentPlanScreen_Query.Data(PlaceholderResolver) { me = buildUser {} }
