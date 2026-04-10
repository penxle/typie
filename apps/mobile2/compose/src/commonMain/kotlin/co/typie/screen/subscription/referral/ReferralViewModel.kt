package co.typie.screen.subscription.referral

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.ReferralScreen_IssueReferralUrl_Mutation
import co.typie.graphql.ReferralScreen_Query
import co.typie.graphql.executeMutation
import co.typie.graphql.type.buildUser
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.result
import com.apollographql.apollo.ApolloClient
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class ReferralViewModel(
  private val apolloClient: ApolloClient,
) : ViewModel() {
  val query = apolloClient.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) { ReferralScreen_Query() }

  suspend fun issueReferralInviteMessage(): Result<String, Nothing> = result {
    val data = apolloClient.executeMutation(ReferralScreen_IssueReferralUrl_Mutation())
    buildReferralInviteMessage(data.issueReferralUrl)
  }
}

private fun buildReferralInviteMessage(url: String): String {
  return "📝 타이피 가입하고 한달 무료 혜택 받아가세요! $url"
}

private fun placeholderData() = ReferralScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    referrals = emptyList()
  }
}
