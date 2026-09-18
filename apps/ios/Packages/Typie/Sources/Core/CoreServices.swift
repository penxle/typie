import Apollo
import Foundation

public struct CoreServices: Sendable {
  public let authState: AuthStateStore
  public let authService: AuthService
  public let activeSite: ActiveSiteStore
  public let client: GraphQLClient
  public let emailLogin: EmailLogin
  public let singleSignOnLogin: SingleSignOnLogin
  public let createSite: CreateSite
  public let serverProbe: ServerProbe?
  public let preferences: UserScopedDefaults
}

public enum CoreAssembly {
  @MainActor
  public static func make(
    config: AppConfig, deviceID: String,
    deviceHeaders: @escaping @Sendable () -> [String: String]
  ) -> CoreServices {
    make(
      config: config, deviceID: deviceID, deviceHeaders: deviceHeaders,
      secureStore: KeychainStore(), preferences: UserScopedDefaults(), store: ApolloStore(),
      configuration: HTTPSession.configuration())
  }

  @MainActor
  static func make(
    config: AppConfig, deviceID: String,
    deviceHeaders: @escaping @Sendable () -> [String: String],
    secureStore: any SecureStore, preferences: UserScopedDefaults, store: ApolloStore,
    configuration: URLSessionConfiguration
  ) -> CoreServices {
    let session = HTTPSession.make(configuration: configuration)
    let authState = AuthStateStore()
    let activeSite = ActiveSiteStore(preferences: preferences)

    let authService = AuthService(
      secureStore: secureStore,
      preferences: preferences,
      authState: authState,
      oidc: OIDCClient(config: config, session: session),
      activeSite: activeSite,
      clearGraphQLCache: { try? await store.clearCache() })

    let client = GraphQLClient.make(
      config: config,
      deviceHeaders: deviceHeaders,
      accessToken: { authService.accessToken },
      onSessionCookie: { try await authService.login(sessionToken: $0) },
      store: store,
      configuration: configuration)

    #if DEBUG
      let serverProbe: ServerProbe? = ServerProbe(
        config: config, client: client, session: session, deviceID: deviceID)
    #else
      let serverProbe: ServerProbe? = nil
    #endif

    return CoreServices(
      authState: authState,
      authService: authService,
      activeSite: activeSite,
      client: client,
      emailLogin: EmailLogin(client: client),
      singleSignOnLogin: SingleSignOnLogin(client: client),
      createSite: CreateSite(client: client),
      serverProbe: serverProbe,
      preferences: preferences)
  }
}
