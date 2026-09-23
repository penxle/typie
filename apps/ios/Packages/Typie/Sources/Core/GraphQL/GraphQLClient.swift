import Apollo
@_spi(Unsafe) import ApolloAPI
import ApolloWebSocket
import Foundation
import GraphQL
import Logging
import Synchronization

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

  func subscribe<S: GraphQLSubscription>(_ subscription: S) -> AsyncStream<S.Data>

  func refetchWatches(where predicate: (any GraphQLOperation) -> Bool)
}

struct Backoff: Sendable {
  var base: Duration = .seconds(1)
  var cap: Duration = .seconds(10)
  var jitter: Duration = .seconds(1)

  func delay(attempt: Int, random: Double = .random(in: 0..<1)) -> Duration {
    min(base * (1 << min(attempt, 16)), cap) + jitter * random
  }
}

struct ApolloGraphQLClient: GraphQLClient {
  let apollo: ApolloClient
  let connection: SubscriptionConnection
  let watches: WatchRegistry
  let backoff: Backoff
  let logger: Logger

  static func make(
    config: AppConfig, deviceHeaders: @escaping @Sendable () -> [String: String],
    accessToken: @escaping @Sendable () -> String?,
    onSessionCookie: @escaping @Sendable (String) async throws -> Void,
    store: ApolloStore = ApolloStore(),
    configuration: URLSessionConfiguration = HTTPSession.configuration(),
    makeSocket: (@Sendable (URLRequest) -> any PingableWebSocketTask)? = nil,
    fetchTicket: (@Sendable () async throws -> String)? = nil,
    subscriptionTiming: SubscriptionConnection.Timing = SubscriptionConnection.Timing(),
    backoff: Backoff = Backoff(),
    logger: Logger = Logger(label: "co.typie.subscription")
  ) -> ApolloGraphQLClient {
    let http = RequestChainNetworkTransport(
      urlSession: URLSession(
        configuration: configuration, delegate: RedirectBlocker(), delegateQueue: nil),
      interceptorProvider: ClientInterceptorProvider(
        deviceHeaders: deviceHeaders, accessToken: accessToken, onSessionCookie: onSessionCookie),
      store: store,
      endpointURL: config.apiURL.appending(path: "graphql"))
    let watches = WatchRegistry()
    let connection = SubscriptionConnection(
      endpointURL: config.wsURL.appending(path: "graphql"),
      store: store,
      makeTask: makeSocket ?? webSocketTasks(configuration: configuration),
      fetchTicket: fetchTicket ?? { try await createTicket(over: http) },
      onReconnect: { watches.refetch { _ in true } },
      timing: subscriptionTiming,
      logger: logger)
    let transport = SplitNetworkTransport(
      queryTransport: http, mutationTransport: http, subscriptionTransport: connection.transport)
    return ApolloGraphQLClient(
      apollo: ApolloClient(networkTransport: transport, store: store), connection: connection,
      watches: watches, backoff: backoff, logger: logger)
  }

  static func webSocketTasks(configuration: URLSessionConfiguration)
    -> @Sendable (URLRequest) -> any PingableWebSocketTask
  {
    let session = URLSession(configuration: configuration)
    return { request in
      let task: URLSessionWebSocketTask = session.webSocketTask(with: request)
      task.maximumMessageSize = Int(Int32.max)
      return task
    }
  }

  func perform<M: GraphQLMutation>(_ mutation: M) async throws -> M.Data
  where M.ResponseFormat == SingleResponseFormat {
    try Self.outcome(of: try await apollo.perform(mutation: mutation)).get()
  }

  func watch<Q: GraphQLQuery>(
    _ query: Q, onResult: @escaping @Sendable (Result<Q.Data, any Error>) -> Void
  ) -> any QueryWatcher {
    let deferred = DeferredQueryWatcher()
    watches.register(query, deferred)
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

  func subscribe<S: GraphQLSubscription>(_ subscription: S) -> AsyncStream<S.Data> {
    let apollo = apollo
    let connection = connection
    let backoff = backoff
    let logger = logger
    return AsyncStream { continuation in
      let loop = Task {
        await Self.run(
          subscription, apollo: apollo, connection: connection, backoff: backoff, logger: logger
        ) { continuation.yield($0) }
        continuation.finish()
      }
      continuation.onTermination = { _ in loop.cancel() }
    }
  }

  func refetchWatches(where predicate: (any GraphQLOperation) -> Bool) {
    watches.refetch(where: predicate)
  }

  private static func run<S: GraphQLSubscription>(
    _ subscription: S, apollo: ApolloClient, connection: SubscriptionConnection,
    backoff: Backoff, logger: Logger, yield: (S.Data) -> Void
  ) async {
    let metadata: Logger.Metadata = [
      "operation": "\(S.operationName)",
      "variables": "\(subscription.__variables ?? [:])",
    ]
    var attempt = 0
    var acknowledgements = connection.acknowledgements
    while !Task.isCancelled {
      if let epoch = try? await connection.prepare() {
        var failure: (any Error)?
        do {
          for try await response in try apollo.subscribe(
            subscription: subscription, cachePolicy: .networkOnly)
          {
            if let data = response.data { yield(data) }
          }
        } catch WebSocketTransport.Error.graphQLErrors(_) {
          logger.notice("subscription stopped by a GraphQL error", metadata: metadata)
          return
        } catch {
          failure = error
        }
        if Task.isCancelled { return }
        if connection.epoch == epoch {
          if let failure {
            logger.warning(
              "subscription ended with an error",
              metadata: metadata.merging(["error": "\(failure)"]) { $1 })
          } else {
            logger.notice("subscription completed by the server", metadata: metadata)
          }
          return
        }
      }
      let current = connection.acknowledgements
      if current != acknowledgements { attempt = 0 }
      acknowledgements = current
      try? await Task.sleep(for: backoff.delay(attempt: attempt))
      attempt += 1
    }
  }

  private static func createTicket(over http: RequestChainNetworkTransport) async throws -> String {
    let responses = try http.send(
      mutation: SubscriptionConnection_CreateWsSession_Mutation(),
      requestConfiguration: RequestConfiguration(writeResultsToCache: false))
    for try await response in responses {
      return try outcome(of: response).get().createWsSession
    }
    throw HTTPError.malformedResponse(
      "SubscriptionConnection_CreateWsSession_Mutation: no response")
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

final class WatchRegistry: Sendable {
  private struct Entry {
    let operation: any GraphQLOperation
    weak var watcher: DeferredQueryWatcher?
  }

  private let entries = Mutex<[Entry]>([])

  var count: Int { entries.withLock { $0.count } }

  func register(_ operation: any GraphQLOperation, _ watcher: DeferredQueryWatcher) {
    entries.withLock { entries in
      entries.removeAll { $0.watcher?.isCancelled ?? true }
      entries.append(Entry(operation: operation, watcher: watcher))
    }
  }

  func refetch(where predicate: (any GraphQLOperation) -> Bool) {
    let live = entries.withLock {
      entries -> [(operation: any GraphQLOperation, watcher: DeferredQueryWatcher)] in
      entries.removeAll { $0.watcher?.isCancelled ?? true }
      return entries.compactMap { entry in entry.watcher.map { (entry.operation, $0) } }
    }
    for (operation, watcher) in live where predicate(operation) {
      Task { await watcher.refetch() }
    }
  }
}

final class DeferredQueryWatcher: QueryWatcher, @unchecked Sendable {
  private let lock = NSLock()
  private var attached: (any QueryWatcher)?
  private var cancelled = false
  private var refetchRequested = false

  var isCancelled: Bool { lock.withLock { cancelled } }

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
