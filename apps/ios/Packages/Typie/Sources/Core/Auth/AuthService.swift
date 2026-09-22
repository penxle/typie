import Synchronization

protocol OIDCExchanging: Sendable {
  func exchange(sessionToken: String) async throws -> String
  func logout(sessionToken: String) async
  func fetchMe(accessToken: String) async throws -> Me
}

extension OIDCClient: OIDCExchanging {}

public final class AuthService: Sendable {
  private let secureStore: any SecureStore
  private let authState: any AuthStatePublishing
  private let oidc: any OIDCExchanging
  private let clearGraphQLCache: @Sendable () async -> Void

  private let lock = AsyncLock()
  private let currentAccessToken = Mutex<String?>(nil)

  init(
    secureStore: any SecureStore,
    authState: any AuthStatePublishing,
    oidc: any OIDCExchanging,
    clearGraphQLCache: @escaping @Sendable () async -> Void = {}
  ) {
    self.secureStore = secureStore
    self.authState = authState
    self.oidc = oidc
    self.clearGraphQLCache = clearGraphQLCache
  }

  public var accessToken: String? {
    currentAccessToken.withLock { $0 }
  }

  public func login(sessionToken: String) async throws {
    try await lock.withLock {
      await self.clearGraphQLCache()
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

  public func logout() async throws {
    try await Task {
      try await self.lock.withLock {
        if let sessionToken = self.storedTokens()?.sessionToken {
          await self.oidc.logout(sessionToken: sessionToken)
        }
        try await self.unauthenticate()
      }
    }.value
  }

  private func authenticateOrReset(sessionToken: String) async throws {
    do {
      try await authenticate(sessionToken: sessionToken)
    } catch let error as InvalidCredentialsError {
      try? await unauthenticate()
      throw error
    }
  }

  private func authenticate(sessionToken: String) async throws {
    let accessToken = try await oidc.exchange(sessionToken: sessionToken)

    let previousTokens = storedTokens()

    let userId: String
    if previousTokens?.sessionToken == sessionToken, let reusableUserId = previousTokens?.userId {
      userId = reusableUserId
    } else {
      userId = try await oidc.fetchMe(accessToken: accessToken).id
    }

    let tokens = AuthTokens(
      sessionToken: sessionToken, accessToken: accessToken, userId: userId)
    try secureStore.setAuthTokens(tokens)
    await publish(.authenticated(tokens))
  }

  private func unauthenticate() async throws {
    let cleared = Result { try secureStore.setAuthTokens(nil) }
    await publish(.unauthenticated)
    await clearGraphQLCache()
    try cleared.get()
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
