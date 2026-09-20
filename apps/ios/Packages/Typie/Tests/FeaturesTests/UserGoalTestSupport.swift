import ApolloTestSupport
import GraphQL
import GraphQLMocks

@testable import Features

func goalUser(target: Int?, history: [(String, Int, Bool)], today: (String, Int)) async
  -> UserGoalSection_user
{
  await UserGoalSection_user.from(
    goalUserMock(
      target: target,
      history: history.map { (date: $0.0, additions: $0.1, achieved: $0.2) },
      today: (date: today.0, additions: today.1)))
}
