package co.typie.screen.settings.settings

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.domain.auth.AuthService
import co.typie.graphql.Apollo
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.SettingsScreen_Query
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildUser
import co.typie.graphql.watchQuery
import co.typie.platform.PlatformModule

class SettingsViewModel : ViewModel() {
  val authService = AuthService
  val deviceInfo = PlatformModule.deviceInfo
  val query =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) {
      SettingsScreen_Query()
    }
}

private fun placeholderData() = SettingsScreen_Query.Data(PlaceholderResolver) { me = buildUser {} }
