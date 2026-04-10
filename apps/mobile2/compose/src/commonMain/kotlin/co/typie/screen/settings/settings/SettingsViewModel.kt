package co.typie.screen.settings.settings

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.auth.AuthService
import co.typie.graphql.Apollo
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.SettingsScreen_Query
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildUser
import co.typie.graphql.watchQuery
import co.typie.platform.PlatformModule
import co.typie.service.CurrentSubscriptionStore
import co.typie.service.DeveloperPreferencesService
import co.typie.service.SubscriptionService

class SettingsViewModel : ViewModel() {
  val authService = AuthService
  val deviceInfo = PlatformModule.deviceInfo
  val developerPreferences = DeveloperPreferencesService
  val currentSubscriptionStore = CurrentSubscriptionStore
  val subscriptionService = SubscriptionService
  val query =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) {
      SettingsScreen_Query()
    }
}

private fun placeholderData() =
  SettingsScreen_Query.Data(PlaceholderResolver) { me = buildUser { subscription = null } }
