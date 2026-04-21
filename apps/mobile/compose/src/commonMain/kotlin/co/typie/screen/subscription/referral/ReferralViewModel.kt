package co.typie.screen.subscription.referral

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.Apollo
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.ReferralScreen_IssueReferralUrl_Mutation
import co.typie.graphql.ReferralScreen_Query
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildUser
import co.typie.graphql.executeMutation
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.result

class ReferralViewModel : ViewModel() {
  val query =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) {
      ReferralScreen_Query()
    }

  suspend fun issueReferralUrl(): Result<String, Nothing> = result {
    val data = Apollo.executeMutation(ReferralScreen_IssueReferralUrl_Mutation())
    data.issueReferralUrl
  }
}

private fun placeholderData() =
  ReferralScreen_Query.Data(PlaceholderResolver) { me = buildUser { referrals = emptyList() } }
