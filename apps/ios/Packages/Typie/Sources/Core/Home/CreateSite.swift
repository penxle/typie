import GraphQL

public struct CreateSite: Sendable {
  private let client: GraphQLClient

  init(client: GraphQLClient) {
    self.client = client
  }

  public func callAsFunction(name: String) async throws -> String {
    let response = try await client.apollo.perform(
      mutation: SpaceSwitcher_CreateSite_Mutation(input: CreateSiteInput(name: name)))
    if let error = response.errors?.first {
      throw mappedGraphQLError(error)
    }
    guard let id = response.data?.createSite.id else {
      throw HTTPError.malformedResponse("createSite: no data")
    }
    return id
  }
}
