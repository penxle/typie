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

  suspend fun issueReferralInviteMessage(): Result<String, Nothing> = result {
    val data = Apollo.executeMutation(ReferralScreen_IssueReferralUrl_Mutation())
    buildReferralInviteMessage(data.issueReferralUrl)
  }
}

private fun buildReferralInviteMessage(url: String): String {
  return "📝 타이피 가입하고 한달 무료 혜택 받아가세요! $url"
}

private fun placeholderData() =
  ReferralScreen_Query.Data(PlaceholderResolver) { me = buildUser { referrals = emptyList() } }
