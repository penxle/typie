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

public protocol QueryWatcher: Sendable {
  func refetch() async
  func cancel()
}

public protocol GraphQLClient: Sendable {
  func perform<M: GraphQLMutation>(_ mutation: M) async throws -> M.Data
  where M.ResponseFormat == SingleResponseFormat

  func watch<Q: GraphQLQuery>(
    _ query: Q, onResult: @escaping @Sendable (Result<Q.Data, any Error>) -> Void
  ) async -> any QueryWatcher
}

struct ApolloGraphQLClient: GraphQLClient {
  let apollo: ApolloClient

  static func make(
    config: AppConfig, deviceHeaders: @escaping @Sendable () -> [String: String],
    accessToken: @escaping @Sendable () -> String?,
    onSessionCookie: @escaping @Sendable (String) async throws -> Void,
    store: ApolloStore = ApolloStore(),
    configuration: URLSessionConfiguration = HTTPSession.configuration()
  ) -> ApolloGraphQLClient {
    let transport = RequestChainNetworkTransport(
      urlSession: URLSession(
        configuration: configuration, delegate: RedirectBlocker(), delegateQueue: nil),
      interceptorProvider: ClientInterceptorProvider(
        deviceHeaders: deviceHeaders, accessToken: accessToken, onSessionCookie: onSessionCookie),
      store: store,
      endpointURL: config.apiURL.appending(path: "graphql"))
    return ApolloGraphQLClient(apollo: ApolloClient(networkTransport: transport, store: store))
  }

  func perform<M: GraphQLMutation>(_ mutation: M) async throws -> M.Data
  where M.ResponseFormat == SingleResponseFormat {
    try Self.outcome(of: try await apollo.perform(mutation: mutation)).get()
  }

  func watch<Q: GraphQLQuery>(
    _ query: Q, onResult: @escaping @Sendable (Result<Q.Data, any Error>) -> Void
  ) async -> any QueryWatcher {
    let watcher = await apollo.watch(query: query, cachePolicy: .cacheAndNetwork) { result in
      switch result {
      case .success(let response): onResult(Self.outcome(of: response))
      case .failure(let error): onResult(.failure(error))
      }
    }
    return ApolloQueryWatcher(watcher: watcher)
  }

  private static func outcome<O: GraphQLOperation>(of response: GraphQLResponse<O>)
    -> Result<O.Data, any Error>
  {
    if let error = response.errors?.first {
      return .failure(mappedGraphQLError(error))
    }
    guard let data = response.data else {
      return .failure(HTTPError.malformedResponse("\(O.operationName): no data"))
    }
    return .success(data)
  }
}

private struct ApolloQueryWatcher<Q: GraphQLQuery>: QueryWatcher {
  let watcher: GraphQLQueryWatcher<Q>

  func refetch() async {
    await watcher.fetch(fetchBehavior: .NetworkOnly)
  }

  func cancel() {
    watcher.cancel()
  }
}
