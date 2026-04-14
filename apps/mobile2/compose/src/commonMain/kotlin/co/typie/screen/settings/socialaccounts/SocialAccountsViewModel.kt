package co.typie.screen.settings.socialaccounts

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.Apollo
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.SocialAccountsScreen_Query
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildUser
import co.typie.graphql.watchQuery

internal class SocialAccountsViewModel : ViewModel() {
  val query =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) {
      SocialAccountsScreen_Query()
    }
}

private fun placeholderData() =
  SocialAccountsScreen_Query.Data(PlaceholderResolver) {
    me = buildUser { singleSignOns = emptyList() }
  }
