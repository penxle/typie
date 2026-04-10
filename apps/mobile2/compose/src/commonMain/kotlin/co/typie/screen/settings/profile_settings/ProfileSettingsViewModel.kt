package co.typie.screen.settings.profile_settings

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.ProfileSettingsScreen_Query
import co.typie.graphql.ProfileSettingsScreen_UpdateMarketingConsent_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.type.UpdateMarketingConsentInput
import co.typie.graphql.type.buildUser
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.result
import com.apollographql.apollo.ApolloClient
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class ProfileSettingsViewModel(
  private val apolloClient: ApolloClient,
) : ViewModel() {
  val query = apolloClient.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) { ProfileSettingsScreen_Query() }

  suspend fun updateMarketingConsent(marketingConsent: Boolean): Result<Unit, Nothing> = result {
    apolloClient.executeMutation(
      ProfileSettingsScreen_UpdateMarketingConsent_Mutation(
        input = UpdateMarketingConsentInput(marketingConsent = marketingConsent),
      ),
    )
  }
}

private fun placeholderData() = ProfileSettingsScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    marketingConsent = false
  }
}
