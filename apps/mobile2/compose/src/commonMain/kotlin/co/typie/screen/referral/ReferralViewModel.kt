package co.typie.screen.referral

import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.ReferralScreen_IssueReferralUrl_Mutation
import co.typie.graphql.ReferralScreen_Query
import co.typie.graphql.type.buildUser
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class ReferralViewModel : GraphQLViewModel() {
  val query = watchQuery(placeholderData()) { ReferralScreen_Query() }

  suspend fun issueReferralInviteMessage(): String {
    val result = executeMutation(ReferralScreen_IssueReferralUrl_Mutation())
    return buildReferralInviteMessage(result.issueReferralUrl)
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
