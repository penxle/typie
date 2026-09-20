import Apollo
import Foundation
import GraphQL
import Testing

@testable import Core

final class GraphQLStub: StubURLProtocol, @unchecked Sendable {
  private static let lock = NSLock()
  nonisolated(unsafe) private static var recorded: URLRequest?

  static var lastRequest: URLRequest? {
    lock.withLock { recorded }
  }

  override func startLoading() {
    if request.url?.host() == "redirect.example.test" {
      respond(status: 302, headers: ["Location": "https://api.example.test/followed"])
      return
    }
    Self.lock.withLock { Self.recorded = request }
    respond(
      headers: ["Content-Type": "application/json"],
      body: Data(#"{"data":{"me":{"__typename":"User","id":"probe"}}}"#.utf8))
  }
}

final class PerformStub: StubURLProtocol, @unchecked Sendable {
  private static let lock = NSLock()
  nonisolated(unsafe) private static var responseBody = "{}"

  static func reset(body: String) {
    lock.withLock { responseBody = body }
  }

  override func startLoading() {
    let body = Self.lock.withLock { Self.responseBody }
    respond(headers: ["Content-Type": "application/json"], body: Data(body.utf8))
  }
}

@Suite(.serialized) struct GraphQLClientTests {
  @Test func sendsDeviceHeadersToGraphQLEndpoint() async throws {
    let client = ApolloGraphQLClient.make(
      config: makeTestConfig(),
      deviceHeaders: { DeviceInfo(id: "abc", model: "iPhone", systemName: "iOS").headers },
      accessToken: { nil }, onSessionCookie: { _ in },
      configuration: stubbedConfiguration(GraphQLStub.self))
    let response = try await client.apollo.fetch(
      query: Ping_Query(), cachePolicy: .networkOnly)
    #expect(response.data?.me?.id == "probe")
    let request = try #require(GraphQLStub.lastRequest)
    #expect(request.url?.absoluteString == "https://api.example.test/graphql")
    #expect(request.value(forHTTPHeaderField: "X-Device-Id") == "abc")
    #expect(request.value(forHTTPHeaderField: "X-Device-Name") == "iPhone")
    #expect(request.value(forHTTPHeaderField: "X-Device-Platform") == "IOS")
  }

  @Test func redirectBlockerStopsRedirects() async throws {
    let url = URL(string: "https://redirect.example.test/start")!
    let blocked = URLSession(
      configuration: stubbedConfiguration(GraphQLStub.self), delegate: RedirectBlocker(),
      delegateQueue: nil)
    let (_, blockedResponse) = try await blocked.data(from: url)
    #expect((blockedResponse as? HTTPURLResponse)?.statusCode == 302)

    let following = URLSession(configuration: stubbedConfiguration(GraphQLStub.self))
    let (_, followedResponse) = try await following.data(from: url)
    #expect((followedResponse as? HTTPURLResponse)?.statusCode == 200)
    #expect(followedResponse.url?.path() == "/followed")
  }

  @Test func performMapsGraphQLErrorsToAPIError() async throws {
    PerformStub.reset(
      body:
        #"{"data":null,"errors":[{"message":"server message","extensions":{"type":"TypieError","code":"rate_limited","message":"too many"}}]}"#
    )
    let client = makePerformClient()
    await #expect(throws: APIError(code: "rate_limited", message: "too many")) {
      _ = try await client.perform(loginMutation())
    }
  }

  @Test func performFailsWhenTheResponseCarriesNoData() async throws {
    PerformStub.reset(body: #"{"data":null}"#)
    let client = makePerformClient()
    await #expect(
      throws: HTTPError.malformedResponse("EmailLogin_LoginWithEmail_Mutation: no data")
    ) {
      _ = try await client.perform(loginMutation())
    }
  }

  private func makePerformClient() -> ApolloGraphQLClient {
    ApolloGraphQLClient.make(
      config: makeTestConfig(), deviceHeaders: { [:] }, accessToken: { nil },
      onSessionCookie: { _ in }, configuration: stubbedConfiguration(PerformStub.self))
  }

  private func loginMutation() -> EmailLogin_LoginWithEmail_Mutation {
    EmailLogin_LoginWithEmail_Mutation(
      input: LoginWithEmailInput(email: "a@b.test", password: "x"))
  }
}
