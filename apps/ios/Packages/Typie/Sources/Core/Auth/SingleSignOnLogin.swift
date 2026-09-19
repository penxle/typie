import GraphQL

public struct SingleSignOnLogin: Sendable {
  private let client: GraphQLClient

  init(client: GraphQLClient) {
    self.client = client
  }

  public func callAsFunction(_ credential: SingleSignOnCredential) async throws {
    let input = AuthorizeSingleSignOnInput(
      params: JSON(credential.params), provider: .case(Self.wireProvider(credential.provider)))
    let response = try await client.apollo.perform(
      mutation: SingleSignOnLogin_AuthorizeSingleSignOn_Mutation(input: input))
    guard let error = response.errors?.first else {
      guard response.data != nil else {
        throw HTTPError.malformedResponse("authorizeSingleSignOn: no data")
      }
      return
    }
    throw mappedGraphQLError(error)
  }

  private static func wireProvider(_ provider: SingleSignOnProvider) -> GraphQL.SingleSignOnProvider
  {
    switch provider {
    case .google: .google
    case .kakao: .kakao
    case .naver: .naver
    case .apple: .apple
    }
  }
}
