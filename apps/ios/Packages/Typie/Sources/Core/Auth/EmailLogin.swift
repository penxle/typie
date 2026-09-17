import GraphQL

public enum EmailLoginError: Error, Equatable, Sendable {
  case invalidCredentials
  case passwordNotSet
}

public struct EmailLogin: Sendable {
  private let client: GraphQLClient

  init(client: GraphQLClient) {
    self.client = client
  }

  public func callAsFunction(email: String, password: String) async throws {
    let response = try await client.apollo.perform(
      mutation: LoginWithEmailMutation(
        input: LoginWithEmailInput(email: email, password: password)))
    if let error = response.errors?.first {
      let mapped = mappedGraphQLError(error)
      guard let typieError = mapped as? TypieError else { throw mapped }
      switch typieError.code {
      case "invalid_credentials": throw EmailLoginError.invalidCredentials
      case "password_not_set": throw EmailLoginError.passwordNotSet
      default: throw typieError
      }
    }
    guard response.data != nil else {
      throw HTTPError.malformedResponse("loginWithEmail: no data")
    }
  }
}
