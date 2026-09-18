import Synchronization

protocol OIDCExchanging: Sendable {
  func exchange(sessionToken: String) async throws -> String
  func logout(sessionToken: String) async
  func fetchMe(accessToken: String) async throws -> Me
}

extension OIDCClient: OIDCExchanging {}

protocol UserScopedPreferences: AnyObject, Sendable {
  func switchUser(_ userId: String?)
  var siteId: String? { get set }
}

extension UserScopedDefaults: UserScopedPreferences {}

public final class AuthService: Sendable {
  private let secureStore: any SecureStore
  private let preferences: any UserScopedPreferences
  private let authState: any AuthStatePublishing
  private let oidc: any OIDCExchanging
  private let activeSite: any ActiveSitePublishing
  private let editingSessions: any EditingSessionRegistry
  private let orphanSweeper: any OrphanSweeping
  private let sync: any SyncConnectionLifecycle
  private let clearGraphQLCache: @Sendable () async -> Void
  private let disconnectSubscriptions: @Sendable () async -> Void
  private let discardEntitlementCache: @Sendable () async -> Void

  private let lock = AsyncLock()
  private let currentAccessToken = Mutex<String?>(nil)

  init(
    secureStore: any SecureStore,
    preferences: any UserScopedPreferences,
    authState: any AuthStatePublishing,
    oidc: any OIDCExchanging,
    activeSite: any ActiveSitePublishing = NoopActiveSitePublisher(),
    editingSessions: any EditingSessionRegistry = NoopEditingSessionRegistry(),
    orphanSweeper: any OrphanSweeping = NoopOrphanSweeper(),
    sync: any SyncConnectionLifecycle = NoopSyncConnection(),
    clearGraphQLCache: @escaping @Sendable () async -> Void = {},
    disconnectSubscriptions: @escaping @Sendable () async -> Void = {},
    discardEntitlementCache: @escaping @Sendable () async -> Void = {}
  ) {
    self.secureStore = secureStore
    self.preferences = preferences
    self.authState = authState
    self.oidc = oidc
    self.activeSite = activeSite
    self.editingSessions = editingSessions
    self.orphanSweeper = orphanSweeper
    self.sync = sync
    self.clearGraphQLCache = clearGraphQLCache
    self.disconnectSubscriptions = disconnectSubscriptions
    self.discardEntitlementCache = discardEntitlementCache
  }

  public var accessToken: String? {
    currentAccessToken.withLock { $0 }
  }

  public func login(sessionToken: String) async throws {
    try await lock.withLock {
      try await self.authenticateOrReset(sessionToken: sessionToken)
    }
  }

  public func renew() async throws {
    try await lock.withLock {
      guard let sessionToken = self.storedTokens()?.sessionToken else {
        await self.publish(.unauthenticated)
        return
      }
      try await self.authenticateOrReset(sessionToken: sessionToken)
    }
  }

  public func logout() async {
    await Task {
      await self.drainEditingSessions()
      try? await self.lock.withLock {
        if let sessionToken = self.storedTokens()?.sessionToken {
          await self.oidc.logout(sessionToken: sessionToken)
        }
        await self.unauthenticate()
      }
    }.value
  }

  private func authenticateOrReset(sessionToken: String) async throws {
    do {
      try await authenticate(sessionToken: sessionToken)
    } catch let error as InvalidCredentialsError {
      await unauthenticate()
      throw error
    }
  }

  private func authenticate(sessionToken: String) async throws {
    let accessToken = try await oidc.exchange(sessionToken: sessionToken)

    let previousTokens = storedTokens()
    let previousSessionToken = previousTokens?.sessionToken

    let userId: String
    if previousSessionToken == sessionToken, let reusableUserId = previousTokens?.userId {
      preferences.switchUser(reusableUserId)
      userId = reusableUserId
    } else {
      let me = try await oidc.fetchMe(accessToken: accessToken)
      preferences.switchUser(me.id)
      if let siteId = resolveActiveSiteId(stored: preferences.siteId, available: me.siteIds) {
        preferences.siteId = siteId
      }
      userId = me.id
    }

    if previousSessionToken != sessionToken {
      await discardEntitlementCache()
    }

    let tokens = AuthTokens(
      sessionToken: sessionToken, accessToken: accessToken, userId: userId)
    do {
      try secureStore.setAuthTokens(tokens)
    } catch {
      preferences.switchUser(previousTokens?.userId)
      throw error
    }
    await publish(.authenticated(tokens))
    await activeSite.publish(preferences.siteId)

    if let previousSessionToken, previousSessionToken != sessionToken {
      await disconnectSubscriptions()
      await sync.onSessionChanged()
    }
  }

  private func unauthenticate() async {
    try? secureStore.setAuthTokens(nil)
    await discardEntitlementCache()
    preferences.switchUser(nil)
    await activeSite.publish(nil)
    await publish(.unauthenticated)
    await clearGraphQLCache()
    await disconnectSubscriptions()
    await sync.onSessionChanged()
  }

  @MainActor
  private func drainEditingSessions() async {
    try? await editingSessions.flushSyncAll()
    await editingSessions.stopAll()
    try? await orphanSweeper.sweep(includeOpenDocuments: true, deleteOnSuccess: true)
  }

  private func publish(_ state: AuthState) async {
    currentAccessToken.withLock { token in
      if case .authenticated(let tokens) = state {
        token = tokens.accessToken
      } else {
        token = nil
      }
    }
    await authState.publish(state)
  }

  private func storedTokens() -> AuthTokens? {
    try? secureStore.authTokens()
  }
}
