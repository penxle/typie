import Apollo
import Foundation
import Testing

@testable import Core

final class AssemblyStub: StubURLProtocol, @unchecked Sendable {
  struct Recorded: Sendable {
    let path: String
    let operation: String?
    let authorization: String?
  }

  private static let lock = NSLock()
  nonisolated(unsafe) private static var recorded: [Recorded] = []
  nonisolated(unsafe) private static var sessionCookiePending = true

  static func reset() {
    lock.withLock {
      recorded = []
      sessionCookiePending = true
    }
  }

  static var requests: [Recorded] { lock.withLock { recorded } }

  override func startLoading() {
    let url = request.url!
    let body = Self.body(of: request).flatMap { String(data: $0, encoding: .utf8) } ?? ""
    let operation = ["EmailLogin_LoginWithEmail_Mutation", "AuthService_Me", "ServerProbe_Query"]
      .first { body.contains($0) }
    Self.lock.withLock {
      Self.recorded.append(
        Recorded(
          path: url.path, operation: operation,
          authorization: request.value(forHTTPHeaderField: "Authorization")))
    }

    var statusCode = 200
    var headers = ["Content-Type": "application/json"]
    var payload = "{}"
    switch (url.path, operation) {
    case ("/graphql", "EmailLogin_LoginWithEmail_Mutation"):
      let attachCookie = Self.lock.withLock {
        defer { Self.sessionCookiePending = false }
        return Self.sessionCookiePending
      }
      if attachCookie { headers["Set-Cookie"] = "typie-st=session-1; Path=/; HttpOnly" }
      payload = #"{"data":{"loginWithEmail":true}}"#
    case ("/graphql", "AuthService_Me"):
      payload = #"{"data":{"me":{"id":"user-1","sites":[{"id":"site-1"}]}}}"#
    case ("/graphql", "ServerProbe_Query"):
      payload =
        #"{"data":{"__typename":"Query","randomName":"probe","me":{"__typename":"User","id":"user-1"}}}"#
    case ("/authorize", _):
      statusCode = 302
      headers["Location"] = "typie:///authorize?code=code-1"
    case ("/token", _):
      payload = #"{"access_token":"access-1"}"#
    default:
      break
    }

    respond(status: statusCode, headers: headers, body: Data(payload.utf8))
  }
}

@MainActor
@Suite(.serialized) struct CoreServicesTests {
  private func makeServices(store: ApolloStore = ApolloStore()) throws -> CoreServices {
    let defaults = try #require(UserDefaults(suiteName: "CoreServicesTests.\(UUID().uuidString)"))
    return CoreAssembly.make(
      config: try makeTestConfig(), deviceID: "device-id", deviceHeaders: { [:] },
      secureStore: InMemorySecureStore(), preferences: UserScopedDefaults(defaults: defaults),
      store: store, configuration: stubbedConfiguration(AssemblyStub.self))
  }

  @Test func assemblesUnauthenticatedServices() throws {
    let services = try makeServices()

    #expect(services.authState.state == .unauthenticated)
    #expect(services.authService.accessToken == nil)

    #if DEBUG
      #expect(services.serverProbe?.deviceID == "device-id")
      #expect(services.serverProbe?.apiHost == "api.example.test")
      #expect(services.serverProbe?.authHost == "auth.example.test")
    #endif
  }

  @Test func wiresSessionCookieBearerAndCacheClearing() async throws {
    AssemblyStub.reset()
    let store = ApolloStore()
    let services = try makeServices(store: store)

    try await services.emailLogin(email: "a@b.test", password: "x")

    let expected = AuthTokens(sessionToken: "session-1", accessToken: "access-1", userId: "user-1")
    #expect(services.authState.state == .authenticated(expected))
    #expect(services.authService.accessToken == "access-1")
    #expect(
      AssemblyStub.requests.first { $0.operation == "EmailLogin_LoginWithEmail_Mutation" }?
        .authorization == nil)

    try await services.emailLogin(email: "a@b.test", password: "x")
    #expect(
      AssemblyStub.requests.last { $0.operation == "EmailLogin_LoginWithEmail_Mutation" }?
        .authorization
        == "Bearer access-1")

    #if DEBUG
      let reader = GraphQLClient.make(
        config: try makeTestConfig(), deviceHeaders: { [:] }, accessToken: { nil },
        onSessionCookie: { _ in }, store: store,
        configuration: stubbedConfiguration(AssemblyStub.self))
      let probe = try #require(services.serverProbe)
      #expect(await probe.graphQL() == "randomName=probe me=present")
      let cached = try await reader.apollo.fetch(
        query: ServerProbe_Query(), cachePolicy: .cacheOnly)
      #expect(cached?.data?.randomName == "probe")

      await services.authService.logout()

      #expect(services.authState.state == .unauthenticated)
      let cleared = try await reader.apollo.fetch(
        query: ServerProbe_Query(), cachePolicy: .cacheOnly)
      #expect(cleared?.data == nil)
    #endif
  }
}
