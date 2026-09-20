import Apollo
import ApolloAPI
import Foundation

@testable import Core

final class FakeGraphQLClient: Core.GraphQLClient, @unchecked Sendable {
  struct MissingStub: Error, Equatable {
    let operation: String
  }

  private let lock = NSLock()
  private var performed: [any GraphQLOperation] = []
  private var mutationResults: [String: [Result<Any, any Error>]] = [:]
  private var listeners: [String: [ObjectIdentifier: (Result<Any, any Error>) -> Void]] = [:]
  private var pending: [String: [Result<Any, any Error>]] = [:]
  private var queued: [String: [Result<Any, any Error>]] = [:]
  private var watchCounts: [String: Int] = [:]
  private var refetchCounts: [String: Int] = [:]
  private var gate: Gate?

  func stub<M: GraphQLMutation>(_ results: [Result<M.Data, any Error>], for mutation: M.Type) {
    lock.withLock {
      mutationResults[M.operationName] = results.map { $0.map { $0 as Any } }
    }
  }

  func stub<M: GraphQLMutation>(_ result: Result<M.Data, any Error>, for mutation: M.Type) {
    stub([result], for: mutation)
  }

  func push<Q: GraphQLQuery>(_ result: Result<Q.Data, any Error>, for query: Q.Type) {
    let erased = result.map { $0 as Any }
    let callbacks = lock.withLock { () -> [(Result<Any, any Error>) -> Void] in
      let listeners = listeners[Q.operationName] ?? [:]
      if listeners.isEmpty {
        pending[Q.operationName, default: []].append(erased)
        return []
      }
      return Array(listeners.values)
    }
    for callback in callbacks { callback(erased) }
  }

  func queue<Q: GraphQLQuery>(_ result: Result<Q.Data, any Error>, for query: Q.Type) {
    lock.withLock { queued[Q.operationName, default: []].append(result.map { $0 as Any }) }
  }

  func hold() -> Gate {
    let gate = Gate()
    lock.withLock { self.gate = gate }
    return gate
  }

  var performedMutations: [any GraphQLOperation] { lock.withLock { performed } }

  func performed<M: GraphQLMutation>(_ mutation: M.Type) -> [M] {
    performedMutations.compactMap { $0 as? M }
  }

  func watchCount<Q: GraphQLQuery>(of query: Q.Type) -> Int {
    lock.withLock { watchCounts[Q.operationName] ?? 0 }
  }

  func refetchCount<Q: GraphQLQuery>(of query: Q.Type) -> Int {
    lock.withLock { refetchCounts[Q.operationName] ?? 0 }
  }

  func perform<M: GraphQLMutation>(_ mutation: M) async throws -> M.Data
  where M.ResponseFormat == SingleResponseFormat {
    let (result, gate) = lock.withLock { () -> (Result<Any, any Error>?, Gate?) in
      performed.append(mutation)
      guard var results = mutationResults[M.operationName], !results.isEmpty else {
        return (nil, gate)
      }
      let next = results.removeFirst()
      if !results.isEmpty { mutationResults[M.operationName] = results }
      return (next, gate)
    }
    await gate?.wait()
    guard let result else { throw MissingStub(operation: M.operationName) }
    return try result.get() as! M.Data
  }

  func watch<Q: GraphQLQuery>(
    _ query: Q, onResult: @escaping @Sendable (Result<Q.Data, any Error>) -> Void
  ) async -> any QueryWatcher {
    let token = Token()
    let deliver = { (result: Result<Any, any Error>) in
      onResult(result.map { $0 as! Q.Data })
    }
    let held = lock.withLock { () -> [Result<Any, any Error>] in
      watchCounts[Q.operationName, default: 0] += 1
      listeners[Q.operationName, default: [:]][ObjectIdentifier(token)] = deliver
      return pending.removeValue(forKey: Q.operationName) ?? []
    }
    for result in held { deliver(result) }
    return Watcher(client: self, operation: Q.operationName, token: token)
  }

  fileprivate final class Token: Sendable {}

  private func refetch(_ operation: String) {
    let (callbacks, next) = lock.withLock {
      () -> ([(Result<Any, any Error>) -> Void], Result<Any, any Error>?) in
      refetchCounts[operation, default: 0] += 1
      var results = queued[operation] ?? []
      let next = results.isEmpty ? nil : results.removeFirst()
      queued[operation] = results
      return (Array((listeners[operation] ?? [:]).values), next)
    }
    guard let next else { return }
    for callback in callbacks { callback(next) }
  }

  private func stopWatching(_ operation: String, _ token: Token) {
    lock.withLock {
      _ = listeners[operation]?.removeValue(forKey: ObjectIdentifier(token))
    }
  }

  private struct Watcher: QueryWatcher {
    let client: FakeGraphQLClient
    let operation: String
    let token: Token

    func refetch() async {
      client.refetch(operation)
    }

    func cancel() {
      client.stopWatching(operation, token)
    }
  }
}
