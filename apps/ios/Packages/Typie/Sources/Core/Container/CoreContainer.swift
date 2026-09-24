import Alamofire
import Apollo
import FactoryKit
import Foundation

extension Scope {
  public static let session = Cached()
}

extension Container {
  public var appConfig: Factory<AppConfig> {
    self { AppConfig.load() }.singleton
  }

  public var httpSessionConfiguration: Factory<URLSessionConfiguration> {
    self { HTTPSession.configuration() }.singleton
  }

  public var device: Factory<DeviceInfo> {
    self { fatalError("device must be registered by Platform") }.singleton
  }

  public var singleSignOn:
    Factory<
      @MainActor (SingleSignOnProvider, @escaping @MainActor () -> AnyObject?) ->
        any SingleSignOnAdapter
    >
  {
    self { fatalError("singleSignOn must be registered by Platform") }.singleton
  }

  var httpSession: Factory<Session> {
    self { HTTPSession.make(configuration: self.httpSessionConfiguration()) }.singleton
  }

  var webSocketTaskFactory: Factory<@Sendable (URLRequest) -> any PingableWebSocketTask> {
    self { ApolloGraphQLClient.webSocketTasks(configuration: self.httpSessionConfiguration()) }
      .singleton
  }

  public var secureStore: Factory<any SecureStore> {
    self { KeychainStore() }.singleton
  }

  public var apolloStore: Factory<ApolloStore> {
    self { GraphQLCache.makeStore() }.singleton
  }

  @MainActor public var appLifecycle: Factory<AppLifecycle> {
    self { AppLifecycle() }.singleton
  }

  @MainActor public var authState: Factory<AuthStateStore> {
    self { AuthStateStore() }.singleton
  }

  @MainActor public var authService: Factory<AuthService> {
    self {
      AuthService(
        secureStore: self.secureStore(),
        authState: self.authState(),
        oidc: OIDCClient(config: self.appConfig(), session: self.httpSession()),
        clearGraphQLCache: { [store = self.apolloStore()] in try? await store.clearCache() },
        disconnectSubscriptions: { [container = self] in
          await MainActor.run {
            (container.graphQLClient() as? ApolloGraphQLClient)?.connection.reset()
          }
        })
    }.singleton
  }

  @MainActor public var graphQLClient: Factory<any GraphQLClient> {
    self {
      let device = self.device()
      let auth = self.authService()
      let client = ApolloGraphQLClient.make(
        config: self.appConfig(), deviceHeaders: { device.headers },
        accessToken: { auth.accessToken },
        onSessionCookie: { try await auth.login(sessionToken: $0) },
        store: self.apolloStore(), configuration: self.httpSessionConfiguration(),
        makeSocket: self.webSocketTaskFactory())
      client.connection.follow(self.appLifecycle())
      return client
    }.singleton
  }

  @MainActor public var devMode: Factory<DevModeStore> {
    self { DevModeStore(defaults: .standard) }.singleton
  }

  @MainActor public var userPreferences: Factory<UserPreferences> {
    self {
      guard case .authenticated(let tokens) = self.authState().state else { fatalError() }
      return UserPreferences(userId: tokens.userId)
    }.scope(.session)
  }

  @MainActor public var activeSite: Factory<ActiveSiteStore> {
    self { ActiveSiteStore(preferences: self.userPreferences()) }.scope(.session)
  }
}
