import Apollo
import ApolloAPI
import Foundation

struct ClientInterceptorProvider: InterceptorProvider {
  let deviceHeaders: @Sendable () -> [String: String]
  let accessToken: @Sendable () -> String?
  let onSessionCookie: @Sendable (String) async throws -> Void

  func graphQLInterceptors<Operation: GraphQLOperation>(for operation: Operation)
    -> [any GraphQLInterceptor]
  {
    DefaultInterceptorProvider.shared.graphQLInterceptors(for: operation)
      + [
        DeviceHeadersInterceptor(headers: deviceHeaders),
        BearerInterceptor(accessToken: accessToken),
      ]
  }

  func httpInterceptors<Operation: GraphQLOperation>(for operation: Operation)
    -> [any HTTPInterceptor]
  {
    [SessionCookieInterceptor(onSessionCookie: onSessionCookie)]
      + DefaultInterceptorProvider.shared.httpInterceptors(for: operation)
  }
}

final class RedirectBlocker: NSObject, URLSessionTaskDelegate, Sendable {
  func urlSession(
    _ session: URLSession, task: URLSessionTask,
    willPerformHTTPRedirection response: HTTPURLResponse, newRequest request: URLRequest
  ) async -> URLRequest? {
    nil
  }
}

struct GraphQLClient: Sendable {
  let apollo: ApolloClient

  static func make(
    config: AppConfig, deviceHeaders: @escaping @Sendable () -> [String: String],
    accessToken: @escaping @Sendable () -> String?,
    onSessionCookie: @escaping @Sendable (String) async throws -> Void,
    store: ApolloStore = ApolloStore(),
    configuration: URLSessionConfiguration = HTTPSession.configuration()
  ) -> GraphQLClient {
    let transport = RequestChainNetworkTransport(
      urlSession: URLSession(
        configuration: configuration, delegate: RedirectBlocker(), delegateQueue: nil),
      interceptorProvider: ClientInterceptorProvider(
        deviceHeaders: deviceHeaders, accessToken: accessToken, onSessionCookie: onSessionCookie),
      store: store,
      endpointURL: config.apiURL.appending(path: "graphql"))
    return GraphQLClient(apollo: ApolloClient(networkTransport: transport, store: store))
  }
}
