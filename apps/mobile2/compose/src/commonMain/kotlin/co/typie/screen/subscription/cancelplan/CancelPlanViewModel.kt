package co.typie.screen.subscription.cancelplan

import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.Apollo
import co.typie.graphql.CancelPlanScreen_Query
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildUser
import co.typie.graphql.watchQuery

internal class CancelPlanViewModel : ViewModel() {
  val query =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) {
      CancelPlanScreen_Query()
    }
}

private fun placeholderData() =
  CancelPlanScreen_Query.Data(PlaceholderResolver) { me = buildUser {} }
