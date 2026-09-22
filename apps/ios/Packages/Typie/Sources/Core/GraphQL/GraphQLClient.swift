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
  ) -> any QueryWatcher
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
  ) -> any QueryWatcher {
    let deferred = DeferredQueryWatcher()
    let apollo = apollo
    Task {
      let watcher = await apollo.watch(query: query, cachePolicy: .cacheAndNetwork) { result in
        switch result {
        case .success(let response): onResult(Self.outcome(of: response))
        case .failure(let error): onResult(.failure(error))
        }
      }
      deferred.attach(ApolloQueryWatcher(watcher: watcher))
    }
    return deferred
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

final class DeferredQueryWatcher: QueryWatcher, @unchecked Sendable {
  private let lock = NSLock()
  private var attached: (any QueryWatcher)?
  private var cancelled = false
  private var refetchRequested = false

  func attach(_ watcher: any QueryWatcher) {
    let (cancelled, refetchRequested) = lock.withLock { () -> (Bool, Bool) in
      attached = watcher
      return (self.cancelled, self.refetchRequested)
    }
    if cancelled {
      watcher.cancel()
    } else if refetchRequested {
      Task { await watcher.refetch() }
    }
  }

  func refetch() async {
    let watcher = lock.withLock { () -> (any QueryWatcher)? in
      if attached == nil { refetchRequested = true }
      return attached
    }
    await watcher?.refetch()
  }

  func cancel() {
    let watcher = lock.withLock { () -> (any QueryWatcher)? in
      cancelled = true
      return attached
    }
    watcher?.cancel()
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
