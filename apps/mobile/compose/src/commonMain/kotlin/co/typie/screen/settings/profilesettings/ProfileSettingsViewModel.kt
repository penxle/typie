package co.typie.screen.settings.profilesettings

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.Apollo
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.ProfileSettingsScreen_Query
import co.typie.graphql.ProfileSettingsScreen_UpdateMarketingConsent_Mutation
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildUser
import co.typie.graphql.executeMutation
import co.typie.graphql.type.UpdateMarketingConsentInput
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.loading

class ProfileSettingsViewModel : ViewModel() {
  val query =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) {
      ProfileSettingsScreen_Query()
    }

  var isUpdatingMarketingConsent by mutableStateOf(false)
    private set

  suspend fun updateMarketingConsent(marketingConsent: Boolean): Result<Unit, Nothing> {
    if (isUpdatingMarketingConsent || query.data.me.marketingConsent == marketingConsent)
      return Result.Ok(Unit)

    return loading({ isUpdatingMarketingConsent = it }) {
      Apollo.executeMutation(
        ProfileSettingsScreen_UpdateMarketingConsent_Mutation(
          input = UpdateMarketingConsentInput(marketingConsent = marketingConsent)
        )
      )
    }
  }
}

private fun placeholderData() =
  ProfileSettingsScreen_Query.Data(PlaceholderResolver) {
    me = buildUser { marketingConsent = false }
  }
