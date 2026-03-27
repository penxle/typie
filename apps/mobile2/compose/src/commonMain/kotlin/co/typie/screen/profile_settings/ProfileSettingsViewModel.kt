package co.typie.screen.profile_settings

import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.ProfileSettingsScreen_Query
import co.typie.graphql.ProfileSettingsScreen_UpdateMarketingConsent_Mutation
import co.typie.graphql.type.UpdateMarketingConsentInput
import co.typie.graphql.type.buildUser
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class ProfileSettingsViewModel : GraphQLViewModel() {
  val query = watchQuery(placeholderData()) { ProfileSettingsScreen_Query() }

  suspend fun updateMarketingConsent(marketingConsent: Boolean) {
    executeMutation(
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
