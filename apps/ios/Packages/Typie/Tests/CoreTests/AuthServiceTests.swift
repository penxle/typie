import Foundation
import Testing

@testable import Core

private struct FakeOIDC: OIDCExchanging {
  let recorder: CallRecorder
  let onExchange: @Sendable (String) async throws -> String
  let onFetchMe: @Sendable (String) async throws -> Me
  let onLogout: @Sendable (String) async -> Void

  func exchange(sessionToken: String) async throws -> String {
    recorder.record("exchange(\(sessionToken))")
    return try await onExchange(sessionToken)
  }

  func fetchMe(accessToken: String) async throws -> Me {
    recorder.record("fetchMe")
    return try await onFetchMe(accessToken)
  }

  func logout(sessionToken: String) async {
    recorder.record("oidcLogout")
    await onLogout(sessionToken)
  }
}

private final class FakeSecureStore: SecureStore, @unchecked Sendable {
  private let store = InMemorySecureStore()
  private let lock = NSLock()
  private var failsWrites = false
  private var failsClears = false
  private let recorder: CallRecorder

  init(recorder: CallRecorder, tokens: AuthTokens? = nil) {
    self.recorder = recorder
    try? store.setAuthTokens(tokens)
  }

  func failWrites() {
    lock.withLock { failsWrites = true }
  }

  func failClears() {
    lock.withLock { failsClears = true }
  }

  func data(for key: String) throws -> Data? {
    try store.data(for: key)
  }

  func set(_ data: Data?, for key: String) throws {
    if data != nil, lock.withLock({ failsWrites }) { throw CocoaError(.fileWriteUnknown) }
    if data == nil, lock.withLock({ failsClears }) { throw CocoaError(.fileWriteUnknown) }
    try store.set(data, for: key)
    recorder.record(data == nil ? "tokenCleared" : "store")
  }
}

private final class FakeAuthStatePublisher: AuthStatePublishing, @unchecked Sendable {
  private let lock = NSLock()
  private var states: [AuthState] = []
  private var snapshots: [String?] = []
  private var snapshotProbe: (@Sendable () -> String?)?
  private let recorder: CallRecorder

  init(recorder: CallRecorder) {
    self.recorder = recorder
  }

  func observeSnapshot(_ probe: @escaping @Sendable () -> String?) {
    lock.withLock { snapshotProbe = probe }
  }

  func publish(_ state: AuthState) async {
    let probe = lock.withLock { snapshotProbe }
    let snapshot = probe?()
    lock.withLock {
      states.append(state)
      snapshots.append(snapshot)
    }
    recorder.record("publish")
  }

  var published: [AuthState] { lock.withLock { states } }
  var snapshotsAtPublish: [String?] { lock.withLock { snapshots } }
}

private struct Harness {
  let recorder: CallRecorder
  let store: FakeSecureStore
  let publisher: FakeAuthStatePublisher
  let service: AuthService

  var calls: [String] { recorder.calls }
}

private func makeHarness(
  tokens: AuthTokens? = nil,
  exchange: @escaping @Sendable (String) async throws -> String = { "access-\($0)" },
  fetchMe: @escaping @Sendable (String) async throws -> Me = { _ in Me(id: "user-1") },
  logout: @escaping @Sendable (String) async -> Void = { _ in }
) -> Harness {
  let recorder = CallRecorder()
  let store = FakeSecureStore(recorder: recorder, tokens: tokens)
  let publisher = FakeAuthStatePublisher(recorder: recorder)
  let service = AuthService(
    secureStore: store,
    authState: publisher,
    oidc: FakeOIDC(
      recorder: recorder, onExchange: exchange, onFetchMe: fetchMe, onLogout: logout),
    clearGraphQLCache: { recorder.record("clearGraphQLCache") },
    disconnectSubscriptions: { recorder.record("disconnectSubscriptions") }
  )
  return Harness(recorder: recorder, store: store, publisher: publisher, service: service)
}

@Suite struct AuthServiceTests {
  @Test func firstLoginFetchesTheUserAndPublishes() async throws {
    let harness = makeHarness()

    try await harness.service.login(sessionToken: "session-1")

    #expect(
      harness.calls == ["clearGraphQLCache", "exchange(session-1)", "fetchMe", "store", "publish"])
    #expect(
      harness.publisher.published == [
        .authenticated(
          AuthTokens(sessionToken: "session-1", accessToken: "access-session-1", userId: "user-1"))
      ])
    #expect(harness.service.accessToken == "access-session-1")
    #expect(try harness.store.authTokens()?.userId == "user-1")
  }

  @Test func renewWithSameSessionTokenReusesStoredUserId() async throws {
    let harness = makeHarness(
      tokens: AuthTokens(
        sessionToken: "session-1", accessToken: "stale", userId: "user-9"),
      fetchMe: { _ in
        Issue.record("fetchMe must not be called")
        return Me(id: "other")
      }
    )

    try await harness.service.renew()

    #expect(harness.calls == ["exchange(session-1)", "store", "publish"])
    #expect(try harness.store.authTokens()?.userId == "user-9")
    #expect(try harness.store.authTokens()?.accessToken == "access-session-1")
    #expect(harness.service.accessToken == "access-session-1")
  }

  @Test func loginClearsTheGraphQLCacheButRenewKeepsIt() async throws {
    let login = makeHarness()
    try await login.service.login(sessionToken: "session-1")
    #expect(login.calls.filter { $0 == "clearGraphQLCache" }.count == 1)

    let renew = makeHarness(
      tokens: AuthTokens(sessionToken: "session-1", accessToken: "stale", userId: "user-9"))
    try await renew.service.renew()
    #expect(renew.calls.contains("clearGraphQLCache") == false)
  }

  @Test func loginWithDifferentSessionTokenReplacesTheStoredSession() async throws {
    let harness = makeHarness(
      tokens: AuthTokens(sessionToken: "session-old", accessToken: "old", userId: "user-old"),
      fetchMe: { _ in Me(id: "user-new") }
    )

    try await harness.service.login(sessionToken: "session-new")

    #expect(
      harness.calls == [
        "clearGraphQLCache", "exchange(session-new)", "fetchMe", "store", "publish",
        "disconnectSubscriptions",
      ])
    #expect(try harness.store.authTokens()?.userId == "user-new")
  }

  @Test func fetchMeFailureLeavesPreviousSessionIntact() async throws {
    let harness = makeHarness(
      tokens: AuthTokens(sessionToken: "session-old", accessToken: "old", userId: "user-old"),
      fetchMe: { _ in throw HTTPError.status(500) }
    )

    await #expect(throws: HTTPError.status(500)) {
      try await harness.service.login(sessionToken: "session-new")
    }

    #expect(harness.calls == ["clearGraphQLCache", "exchange(session-new)", "fetchMe"])
    #expect(harness.publisher.published.isEmpty)
    #expect(try harness.store.authTokens()?.sessionToken == "session-old")
    #expect(harness.service.accessToken == nil)
  }

  @Test func invalidCredentialsUnauthenticatesInOrder() async throws {
    let harness = makeHarness(
      tokens: AuthTokens(sessionToken: "session-old", accessToken: "old", userId: "user-old"),
      exchange: { _ in throw InvalidCredentialsError() }
    )

    await #expect(throws: InvalidCredentialsError()) {
      try await harness.service.login(sessionToken: "session-new")
    }

    #expect(
      harness.calls == [
        "clearGraphQLCache", "exchange(session-new)", "tokenCleared", "publish",
        "clearGraphQLCache", "disconnectSubscriptions",
      ])
    #expect(harness.publisher.published == [.unauthenticated])
    #expect(try harness.store.authTokens() == nil)
    #expect(harness.service.accessToken == nil)
  }

  @Test func transportFailuresKeepTheStoredSession() async throws {
    for failure in [HTTPError.network("offline"), HTTPError.status(500)] {
      let harness = makeHarness(
        tokens: AuthTokens(sessionToken: "session-1", accessToken: "old", userId: "user-1"),
        exchange: { _ in throw failure }
      )

      await #expect(throws: failure) { try await harness.service.renew() }

      #expect(harness.calls == ["exchange(session-1)"])
      #expect(try harness.store.authTokens()?.accessToken == "old")
      #expect(harness.publisher.published.isEmpty)
    }
  }

  @Test func renewWithoutStoredTokensOnlyPublishesUnauthenticated() async throws {
    let harness = makeHarness()

    try await harness.service.renew()

    #expect(harness.calls == ["publish"])
    #expect(harness.publisher.published == [.unauthenticated])
    #expect(harness.service.accessToken == nil)
  }

  @Test func logoutClearsEverythingInOrder() async throws {
    let harness = makeHarness(
      tokens: AuthTokens(sessionToken: "session-1", accessToken: "old", userId: "user-1"))

    try await harness.service.logout()

    #expect(
      harness.calls == [
        "oidcLogout", "tokenCleared", "publish", "clearGraphQLCache", "disconnectSubscriptions",
      ])
    #expect(harness.publisher.published == [.unauthenticated])
    #expect(try harness.store.authTokens() == nil)
  }

  @Test func logoutWithoutStoredSessionSkipsTheOIDCCall() async throws {
    let harness = makeHarness()

    try await harness.service.logout()

    #expect(
      harness.calls == ["tokenCleared", "publish", "clearGraphQLCache", "disconnectSubscriptions"])
  }

  @Test func logoutThrowsWhenClearingTheKeychainFails() async throws {
    let harness = makeHarness(
      tokens: AuthTokens(sessionToken: "session-1", accessToken: "old", userId: "user-1"))
    harness.store.failClears()

    await #expect(throws: CocoaError.self) { try await harness.service.logout() }

    #expect(
      harness.calls == ["oidcLogout", "publish", "clearGraphQLCache", "disconnectSubscriptions"])
    #expect(harness.publisher.published == [.unauthenticated])
    #expect(harness.service.accessToken == nil)
  }

  @Test func concurrentLoginsRunOneAfterTheOther() async throws {
    let gate = TestGate()
    let harness = makeHarness(
      exchange: { token in
        if token == "session-1" { await gate.wait() }
        return "access-\(token)"
      },
      fetchMe: { accessToken in Me(id: "user-\(accessToken)") }
    )

    let first = Task { try await harness.service.login(sessionToken: "session-1") }
    try await waitUntil { harness.recorder.calls.contains("exchange(session-1)") }

    let second = Task { try await harness.service.login(sessionToken: "session-2") }

    await gate.open()
    try await first.value
    try await second.value

    let calls = harness.calls
    let firstPublish = try #require(calls.firstIndex(of: "publish"))
    let secondExchange = try #require(calls.firstIndex(of: "exchange(session-2)"))
    #expect(secondExchange > firstPublish)
    #expect(harness.service.accessToken == "access-session-2")
  }

  @Test func publishesThroughTheObservableStore() async throws {
    let recorder = CallRecorder()
    let store = FakeSecureStore(recorder: recorder)
    let stateStore = await AuthStateStore()
    let service = AuthService(
      secureStore: store,
      authState: stateStore,
      oidc: FakeOIDC(
        recorder: recorder,
        onExchange: { "access-\($0)" },
        onFetchMe: { _ in Me(id: "user-1") },
        onLogout: { _ in })
    )

    try await service.login(sessionToken: "session-1")
    var state = await stateStore.state
    #expect(
      state
        == .authenticated(
          AuthTokens(
            sessionToken: "session-1", accessToken: "access-session-1", userId: "user-1")))

    try await service.logout()
    state = await stateStore.state
    #expect(state == .unauthenticated)
  }

  @Test func accessTokenSnapshotIsUpdatedBeforePublication() async throws {
    let harness = makeHarness()
    harness.publisher.observeSnapshot { harness.service.accessToken }
    try await harness.service.login(sessionToken: "session-1")
    #expect(harness.publisher.snapshotsAtPublish == ["access-session-1"])
  }

  @Test func tokenStoreFailureFailsLoginAndKeepsTheStoredSession() async throws {
    let previous = AuthTokens(sessionToken: "session-0", accessToken: "access-0", userId: "user-0")
    let harness = makeHarness(tokens: previous)
    harness.store.failWrites()
    await #expect(throws: CocoaError.self) {
      try await harness.service.login(sessionToken: "session-9")
    }
    #expect(harness.publisher.published.isEmpty)
    #expect(try harness.store.authTokens() == previous)
    #expect(harness.service.accessToken == nil)
  }

  @Test func logoutRunsToCompletionWhenTheCallerIsCancelled() async throws {
    let gate = TestGate()
    let previous = AuthTokens(sessionToken: "session-0", accessToken: "access-0", userId: "user-0")
    let harness = makeHarness(tokens: previous, logout: { _ in await gate.wait() })

    let caller = Task { try await harness.service.logout() }
    try await waitUntil { harness.recorder.calls.contains("oidcLogout") }
    caller.cancel()
    await gate.open()
    try await caller.value

    #expect(
      harness.calls == [
        "oidcLogout", "tokenCleared", "publish", "clearGraphQLCache", "disconnectSubscriptions",
      ])
  }

  @Test func keepingTheSessionDoesNotDisconnectSubscriptions() async throws {
    let first = makeHarness()
    try await first.service.login(sessionToken: "session-1")
    #expect(first.calls.contains("disconnectSubscriptions") == false)

    let renew = makeHarness(
      tokens: AuthTokens(sessionToken: "session-1", accessToken: "stale", userId: "user-9"))
    try await renew.service.renew()
    #expect(renew.calls.contains("disconnectSubscriptions") == false)
  }
}
