import Apollo
import FactoryKit
import FactoryTesting
import Foundation
import Testing

@testable import Core

final class ContainerStub: StubURLProtocol, @unchecked Sendable {
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
    let operation = ["AuthService_Me", "Ping_Query"].first { body.contains($0) }
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
    case ("/graphql", "Ping_Query"):
      let attachCookie = Self.lock.withLock {
        defer { Self.sessionCookiePending = false }
        return Self.sessionCookiePending
      }
      if attachCookie { headers["Set-Cookie"] = "typie-st=session-1; Path=/; HttpOnly" }
      payload = #"{"data":{"__typename":"Query","me":{"__typename":"User","id":"user-1"}}}"#
    case ("/graphql", "AuthService_Me"):
      payload = #"{"data":{"me":{"id":"user-1"}}}"#
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
@Suite(.container, .serialized) struct CoreContainerTests {
  private func register(store: ApolloStore) {
    Container.shared.appConfig.register { makeTestConfig() }
    Container.shared.device.register {
      DeviceInfo(id: "device-id", model: "iPhone", systemName: "iOS")
    }
    Container.shared.secureStore.register { InMemorySecureStore() }
    Container.shared.httpSessionConfiguration.register {
      stubbedConfiguration(ContainerStub.self)
    }
    Container.shared.apolloStore.register { store }
  }

  @Test func resolvesUnauthenticatedServices() {
    register(store: ApolloStore())

    #expect(Container.shared.authState().state == .unauthenticated)
    #expect(Container.shared.authService().accessToken == nil)
  }

  @Test func scopesUserPreferencesToTheAuthenticatedUser() async {
    register(store: ApolloStore())
    let userId = "user-\(UUID().uuidString)"
    let key = "site_id@\(userId)"
    defer { UserDefaults.standard.removeObject(forKey: key) }

    let store = AuthStateStore()
    Container.shared.authState.register { store }
    await store.publish(
      .authenticated(
        AuthTokens(sessionToken: "session", accessToken: "access", userId: userId)))

    Container.shared.userPreferences().siteId = "site-1"

    #expect(UserDefaults.standard.string(forKey: key) == "site-1")
  }

  @Test func seedsActiveSiteFromStoredPreferences() {
    register(store: ApolloStore())
    let defaults = UserDefaults(suiteName: "core-container-\(UUID().uuidString)")!
    let preferences = UserPreferences(userId: "user-1", defaults: defaults)
    preferences.siteId = "site-7"
    Container.shared.userPreferences.register { preferences }

    #expect(Container.shared.activeSite().siteId == "site-7")
  }

  @Test func wiresSessionCookieBearerAndCacheClearing() async throws {
    ContainerStub.reset()
    let store = ApolloStore()
    register(store: store)

    let client = try #require(Container.shared.graphQLClient() as? ApolloGraphQLClient)
    let authService = Container.shared.authService()
    let authState = Container.shared.authState()

    let probed = try await client.apollo.fetch(query: Ping_Query(), cachePolicy: .networkOnly)
    #expect(probed.data?.me?.id == "user-1")

    let expected = AuthTokens(sessionToken: "session-1", accessToken: "access-1", userId: "user-1")
    #expect(authState.state == .authenticated(expected))
    #expect(authService.accessToken == "access-1")
    #expect(ContainerStub.requests.first { $0.operation == "Ping_Query" }?.authorization == nil)

    _ = try await client.apollo.fetch(query: Ping_Query(), cachePolicy: .networkOnly)
    #expect(
      ContainerStub.requests.last { $0.operation == "Ping_Query" }?.authorization
        == "Bearer access-1")

    let reader = ApolloGraphQLClient.make(
      config: makeTestConfig(), deviceHeaders: { [:] }, accessToken: { nil },
      onSessionCookie: { _ in }, store: store,
      configuration: stubbedConfiguration(ContainerStub.self))
    let cached = try await reader.apollo.fetch(query: Ping_Query(), cachePolicy: .cacheOnly)
    #expect(cached?.data?.me?.id == "user-1")

    try await authService.logout()

    #expect(authState.state == .unauthenticated)
    let cleared = try await reader.apollo.fetch(query: Ping_Query(), cachePolicy: .cacheOnly)
    #expect(cleared?.data == nil)
  }
}
