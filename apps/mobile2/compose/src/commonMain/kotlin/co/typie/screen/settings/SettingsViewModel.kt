package co.typie.screen.settings

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.auth.AuthService
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.SettingsScreen_Query
import co.typie.graphql.type.buildUser
import co.typie.graphql.watchQuery
import co.typie.platform.DeviceInfo
import co.typie.screen.subscription.CurrentSubscriptionStore
import co.typie.screen.subscription.SubscriptionService
import co.typie.service.DeveloperPreferencesService
import com.apollographql.apollo.ApolloClient
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class SettingsViewModel(
  private val apolloClient: ApolloClient,
  val authService: AuthService,
  val deviceInfo: DeviceInfo,
  val developerPreferences: DeveloperPreferencesService,
  val currentSubscriptionStore: CurrentSubscriptionStore,
  val subscriptionService: SubscriptionService,
) : ViewModel() {
  val query = apolloClient.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) { SettingsScreen_Query() }
}

private fun placeholderData() = SettingsScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    subscription = null
  }
}
