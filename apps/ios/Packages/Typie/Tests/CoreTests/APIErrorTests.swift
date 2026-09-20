import Apollo
import Testing

@testable import Core

@Suite struct APIErrorTests {
  private func graphQLError(message: String, extensions: [String: String]) -> GraphQLError {
    GraphQLError(["message": message, "extensions": extensions])
  }

  @Test func fallsBackToTheTopLevelMessageWhenExtensionsCarryNone() {
    let mapped = mappedGraphQLError(
      graphQLError(
        message: "server message", extensions: ["type": "TypieError", "code": "rate_limited"]))

    #expect(mapped as? APIError == APIError(code: "rate_limited", message: "server message"))
  }

  @Test func prefersTheExtensionMessageOverTheTopLevelOne() {
    let mapped = mappedGraphQLError(
      graphQLError(
        message: "server message",
        extensions: ["type": "TypieError", "code": "rate_limited", "message": "too many"]))

    #expect(mapped as? APIError == APIError(code: "rate_limited", message: "too many"))
  }

  @Test func leavesForeignErrorsUnpromoted() {
    let error = graphQLError(message: "server message", extensions: ["type": "Other"])

    #expect(mappedGraphQLError(error) as? APIError == nil)
    #expect(mappedGraphQLError(error) as? GraphQLError == error)
  }

  @Test func leavesTypieErrorsWithoutACodeUnpromoted() {
    let error = graphQLError(message: "server message", extensions: ["type": "TypieError"])

    #expect(mappedGraphQLError(error) as? APIError == nil)
    #expect(mappedGraphQLError(error) as? GraphQLError == error)
  }
}
