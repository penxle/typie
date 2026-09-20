import Apollo
import ApolloAPI
import Foundation
import GraphQL

@testable import Core

final class GatedWatchClient: Core.GraphQLClient, @unchecked Sendable {
  struct UnsupportedOperation: Error, Equatable {
    let operation: String
  }

  let gate = TestGate()

  private let lock = NSLock()
  private let data: Ping_Query.Data
  private var parked = false
  private var refetches = 0

  init(data: Ping_Query.Data) {
    self.data = data
  }

  var isParked: Bool { lock.withLock { parked } }

  var refetchCount: Int { lock.withLock { refetches } }

  func perform<M: GraphQLMutation>(_ mutation: M) async throws -> M.Data
  where M.ResponseFormat == SingleResponseFormat {
    throw UnsupportedOperation(operation: M.operationName)
  }

  func watch<Q: GraphQLQuery>(
    _ query: Q, onResult: @escaping @Sendable (Result<Q.Data, any Error>) -> Void
  ) async -> any QueryWatcher {
    if let data = data as? Q.Data {
      onResult(.success(data))
    } else {
      onResult(.failure(UnsupportedOperation(operation: Q.operationName)))
    }
    lock.withLock { parked = true }
    await gate.wait()
    return Watcher(client: self)
  }

  fileprivate func countRefetch() {
    lock.withLock { refetches += 1 }
  }

  private struct Watcher: QueryWatcher {
    let client: GatedWatchClient

    func refetch() async {
      client.countRefetch()
    }

    func cancel() {}
  }
}
