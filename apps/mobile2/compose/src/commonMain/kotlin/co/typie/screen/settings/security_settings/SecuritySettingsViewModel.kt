package co.typie.screen.settings.security_settings

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.Apollo
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.SecuritySettingsScreen_Query
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildUser
import co.typie.graphql.watchQuery

class SecuritySettingsViewModel : ViewModel() {
  val query =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) {
      SecuritySettingsScreen_Query()
    }
}

private fun placeholderData() =
  SecuritySettingsScreen_Query.Data(PlaceholderResolver) { me = buildUser { hasPassword = true } }
