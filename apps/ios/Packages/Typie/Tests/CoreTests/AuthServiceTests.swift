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
  private let recorder: CallRecorder

  init(recorder: CallRecorder, tokens: AuthTokens? = nil) {
    self.recorder = recorder
    try? store.setAuthTokens(tokens)
  }

  func failWrites() {
    lock.withLock { failsWrites = true }
  }

  func data(for key: String) throws -> Data? {
    try store.data(for: key)
  }

  func set(_ data: Data?, for key: String) throws {
    if data != nil, lock.withLock({ failsWrites }) { throw CocoaError(.fileWriteUnknown) }
    try store.set(data, for: key)
    recorder.record(data == nil ? "tokenCleared" : "store")
  }
}

private final class FakePreferences: UserScopedPreferences, @unchecked Sendable {
  private let lock = NSLock()
  private var boundUser: String?
  private var site: String?
  private let recorder: CallRecorder

  init(recorder: CallRecorder, siteId: String? = nil) {
    self.recorder = recorder
    site = siteId
  }

  func switchUser(_ userId: String?) {
    lock.withLock { boundUser = userId }
    recorder.record("switchUser(\(userId ?? "nil"))")
  }

  var siteId: String? {
    get { lock.withLock { site } }
    set {
      lock.withLock { site = newValue }
      recorder.record("siteId=\(newValue ?? "nil")")
    }
  }

  var currentUser: String? { lock.withLock { boundUser } }
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

private struct FakeEditingSessions: EditingSessionRegistry {
  let recorder: CallRecorder
  let onFlush: @Sendable () async throws -> Void

  func flushSyncAll() async throws {
    recorder.record("flush")
    try await onFlush()
  }

  func stopAll() async {
    recorder.record("stop")
  }
}

private struct FakeOrphanSweeper: OrphanSweeping {
  let recorder: CallRecorder
  let onSweep: @Sendable () async throws -> Void

  func sweep(includeOpenDocuments: Bool, deleteOnSuccess: Bool) async throws {
    recorder.record("sweep(\(includeOpenDocuments),\(deleteOnSuccess))")
    try await onSweep()
  }
}

private struct FakeSyncConnection: SyncConnectionLifecycle {
  let recorder: CallRecorder

  func onSessionChanged() async {
    recorder.record("syncSessionChanged")
  }
}

private struct Harness {
  let recorder: CallRecorder
  let store: FakeSecureStore
  let preferences: FakePreferences
  let publisher: FakeAuthStatePublisher
  let service: AuthService

  var calls: [String] { recorder.calls }
}

private func makeHarness(
  tokens: AuthTokens? = nil,
  siteId: String? = nil,
  exchange: @escaping @Sendable (String) async throws -> String = { "access-\($0)" },
  fetchMe: @escaping @Sendable (String) async throws -> Me = { _ in
    Me(id: "user-1", siteIds: ["site-1"])
  },
  logout: @escaping @Sendable (String) async -> Void = { _ in },
  flush: @escaping @Sendable () async throws -> Void = {},
  sweep: @escaping @Sendable () async throws -> Void = {}
) -> Harness {
  let recorder = CallRecorder()
  let store = FakeSecureStore(recorder: recorder, tokens: tokens)
  let preferences = FakePreferences(recorder: recorder, siteId: siteId)
  let publisher = FakeAuthStatePublisher(recorder: recorder)
  let service = AuthService(
    secureStore: store,
    preferences: preferences,
    authState: publisher,
    oidc: FakeOIDC(
      recorder: recorder, onExchange: exchange, onFetchMe: fetchMe, onLogout: logout),
    editingSessions: FakeEditingSessions(recorder: recorder, onFlush: flush),
    orphanSweeper: FakeOrphanSweeper(recorder: recorder, onSweep: sweep),
    sync: FakeSyncConnection(recorder: recorder),
    clearGraphQLCache: { recorder.record("clearGraphQLCache") },
    disconnectSubscriptions: { recorder.record("disconnectSubscriptions") },
    discardEntitlementCache: { recorder.record("discardEntitlementCache") }
  )
  return Harness(
    recorder: recorder, store: store, preferences: preferences, publisher: publisher,
    service: service)
}

@Suite struct AuthServiceTests {
  @Test func firstLoginResolvesSiteAndPublishesWithoutTouchingSockets() async throws {
    let harness = makeHarness(fetchMe: { _ in Me(id: "user-1", siteIds: ["site-1", "site-2"]) })

    try await harness.service.login(sessionToken: "session-1")

    #expect(
      harness.calls == [
        "exchange(session-1)", "fetchMe", "switchUser(user-1)", "siteId=site-1",
        "discardEntitlementCache", "store", "publish",
      ])
    #expect(
      harness.publisher.published == [
        .authenticated(
          AuthTokens(sessionToken: "session-1", accessToken: "access-session-1", userId: "user-1"))
      ])
    #expect(harness.service.accessToken == "access-session-1")
    #expect(try harness.store.authTokens()?.userId == "user-1")
    #expect(harness.preferences.siteId == "site-1")
  }

  @Test func renewWithSameSessionTokenReusesStoredUserId() async throws {
    let harness = makeHarness(
      tokens: AuthTokens(
        sessionToken: "session-1", accessToken: "stale", userId: "user-9"),
      siteId: "site-9",
      fetchMe: { _ in
        Issue.record("fetchMe must not be called")
        return Me(id: "other", siteIds: [])
      }
    )

    try await harness.service.renew()

    #expect(
      harness.calls == [
        "exchange(session-1)", "switchUser(user-9)", "store", "publish",
      ])
    #expect(harness.preferences.siteId == "site-9")
    #expect(try harness.store.authTokens()?.accessToken == "access-session-1")
    #expect(harness.service.accessToken == "access-session-1")
  }

  @Test func loginWithDifferentSessionTokenSwitchesSessionInOrder() async throws {
    let harness = makeHarness(
      tokens: AuthTokens(sessionToken: "session-old", accessToken: "old", userId: "user-old"),
      siteId: "site-old",
      fetchMe: { _ in Me(id: "user-new", siteIds: ["site-new"]) }
    )

    try await harness.service.login(sessionToken: "session-new")

    #expect(
      harness.calls == [
        "exchange(session-new)", "fetchMe", "switchUser(user-new)", "siteId=site-new",
        "discardEntitlementCache", "store", "publish", "disconnectSubscriptions",
        "syncSessionChanged",
      ])
  }

  @Test func fetchMeFailureLeavesPreviousSessionIntact() async throws {
    let harness = makeHarness(
      tokens: AuthTokens(sessionToken: "session-old", accessToken: "old", userId: "user-old"),
      fetchMe: { _ in throw HTTPError.status(500) }
    )

    await #expect(throws: HTTPError.status(500)) {
      try await harness.service.login(sessionToken: "session-new")
    }

    #expect(harness.calls == ["exchange(session-new)", "fetchMe"])
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
        "exchange(session-new)", "tokenCleared", "discardEntitlementCache", "switchUser(nil)",
        "publish", "clearGraphQLCache", "disconnectSubscriptions", "syncSessionChanged",
      ])
    #expect(harness.publisher.published == [.unauthenticated])
    #expect(try harness.store.authTokens() == nil)
    #expect(harness.service.accessToken == nil)
  }

  @Test func transportFailuresKeepTheStoredSession() async throws {
    for failure in [HTTPError.network("offline"), HTTPError.status(500)] {
      let harness = makeHarness(
        tokens: AuthTokens(sessionToken: "session-1", accessToken: "old", userId: "user-1"),
        siteId: "site-1",
        exchange: { _ in throw failure }
      )

      await #expect(throws: failure) { try await harness.service.renew() }

      #expect(harness.calls == ["exchange(session-1)"])
      #expect(try harness.store.authTokens()?.accessToken == "old")
      #expect(harness.preferences.currentUser == nil)
      #expect(harness.preferences.siteId == "site-1")
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

  @Test func keepsStoredSiteIdWhenResolutionYieldsNothing() async throws {
    let harness = makeHarness(
      siteId: "site-keep", fetchMe: { _ in Me(id: "user-1", siteIds: []) })

    try await harness.service.login(sessionToken: "session-1")

    #expect(
      harness.calls == [
        "exchange(session-1)", "fetchMe", "switchUser(user-1)", "discardEntitlementCache", "store",
        "publish",
      ])
    #expect(harness.preferences.siteId == "site-keep")
  }

  @Test func logoutDrainsSessionsThenClearsEverythingInOrder() async throws {
    let harness = makeHarness(
      tokens: AuthTokens(sessionToken: "session-1", accessToken: "old", userId: "user-1"),
      flush: { throw HTTPError.network("flush failed") },
      sweep: { throw HTTPError.network("sweep failed") }
    )

    await harness.service.logout()

    #expect(
      harness.calls == [
        "flush", "stop", "sweep(true,true)", "oidcLogout", "tokenCleared",
        "discardEntitlementCache", "switchUser(nil)", "publish", "clearGraphQLCache",
        "disconnectSubscriptions", "syncSessionChanged",
      ])
    #expect(harness.publisher.published == [.unauthenticated])
    #expect(try harness.store.authTokens() == nil)
  }

  @Test func logoutWithoutStoredSessionSkipsTheOIDCCall() async throws {
    let harness = makeHarness()

    await harness.service.logout()

    #expect(
      harness.calls == [
        "flush", "stop", "sweep(true,true)", "tokenCleared", "discardEntitlementCache",
        "switchUser(nil)", "publish", "clearGraphQLCache", "disconnectSubscriptions",
        "syncSessionChanged",
      ])
  }

  @Test func concurrentLoginsRunOneAfterTheOther() async throws {
    let gate = TestGate()
    let harness = makeHarness(
      exchange: { token in
        if token == "session-1" { await gate.wait() }
        return "access-\(token)"
      },
      fetchMe: { accessToken in Me(id: "user-\(accessToken)", siteIds: []) }
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
      preferences: FakePreferences(recorder: recorder),
      authState: stateStore,
      oidc: FakeOIDC(
        recorder: recorder,
        onExchange: { "access-\($0)" },
        onFetchMe: { _ in Me(id: "user-1", siteIds: ["site-1"]) },
        onLogout: { _ in })
    )

    try await service.login(sessionToken: "session-1")
    var state = await stateStore.state
    #expect(
      state
        == .authenticated(
          AuthTokens(
            sessionToken: "session-1", accessToken: "access-session-1", userId: "user-1")))

    await service.logout()
    state = await stateStore.state
    #expect(state == .unauthenticated)
  }

  @Test func accessTokenSnapshotIsUpdatedBeforePublication() async throws {
    let harness = makeHarness()
    harness.publisher.observeSnapshot { harness.service.accessToken }
    try await harness.service.login(sessionToken: "session-1")
    #expect(harness.publisher.snapshotsAtPublish == ["access-session-1"])
  }

  @Test func tokenStoreFailureFailsLoginAndKeepsPreviousIdentity() async throws {
    let previous = AuthTokens(sessionToken: "session-0", accessToken: "access-0", userId: "user-0")
    let harness = makeHarness(tokens: previous)
    harness.store.failWrites()
    await #expect(throws: CocoaError.self) {
      try await harness.service.login(sessionToken: "session-9")
    }
    #expect(harness.publisher.published.isEmpty)
    #expect(harness.preferences.currentUser == "user-0")
    #expect(try harness.store.authTokens() == previous)
    #expect(!harness.calls.contains("disconnectSubscriptions"))
  }

  @Test func logoutRunsToCompletionWhenTheCallerIsCancelled() async throws {
    let gate = TestGate()
    let previous = AuthTokens(sessionToken: "session-0", accessToken: "access-0", userId: "user-0")
    let harness = makeHarness(tokens: previous, logout: { _ in await gate.wait() })

    let caller = Task { await harness.service.logout() }
    try await waitUntil { harness.recorder.calls.contains("oidcLogout") }
    caller.cancel()
    await gate.open()
    await caller.value

    #expect(
      harness.calls == [
        "flush", "stop", "sweep(true,true)", "oidcLogout", "tokenCleared",
        "discardEntitlementCache", "switchUser(nil)", "publish", "clearGraphQLCache",
        "disconnectSubscriptions", "syncSessionChanged",
      ])
  }
}
