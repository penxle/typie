package co.typie.screen.settings

import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.SettingsScreen_Query
import co.typie.graphql.SettingsScreen_UpdateMarketingConsent_Mutation
import co.typie.graphql.type.UpdateMarketingConsentInput
import co.typie.graphql.type.buildUser
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class SettingsViewModel : GraphQLViewModel() {
  val query = watchQuery(placeholderData()) { SettingsScreen_Query() }

  suspend fun updateMarketingConsent(marketingConsent: Boolean) {
    executeMutation(
      SettingsScreen_UpdateMarketingConsent_Mutation(
        input = UpdateMarketingConsentInput(marketingConsent = marketingConsent),
      ),
    )
  }
}

private fun placeholderData() = SettingsScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    marketingConsent = false
    hasPassword = true
  }
}
