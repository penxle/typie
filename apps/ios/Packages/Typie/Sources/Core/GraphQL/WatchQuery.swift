import Apollo
import ApolloAPI
import Observation

public struct NoInput: Equatable, Sendable {
  public init() {}
}

@MainActor @Observable
public final class WatchQuery<Input: Equatable & Sendable, Query: GraphQLQuery> {
  public private(set) var data: Query.Data?
  public private(set) var error: (any Error)?
  public private(set) var isSettled = false

  @ObservationIgnored private let client: any GraphQLClient
  @ObservationIgnored private let input: () -> Input?
  @ObservationIgnored private let query: (Input) -> Query
  @ObservationIgnored private let keepsDataOnInputChange: Bool
  @ObservationIgnored private var currentInput: Input?
  @ObservationIgnored private var watcher: (any QueryWatcher)?
  @ObservationIgnored private var consumer: Task<Void, Never>?
  @ObservationIgnored private var generation = 0
  @ObservationIgnored private var refetchPending = false

  public init(
    client: any GraphQLClient,
    input: @escaping () -> Input?,
    query: @escaping (Input) -> Query,
    keepsDataOnInputChange: Bool = false
  ) {
    self.client = client
    self.input = input
    self.query = query
    self.keepsDataOnInputChange = keepsDataOnInputChange
    observeInput()
  }

  public convenience init(client: any GraphQLClient, query: Query) where Input == NoInput {
    self.init(client: client, input: { NoInput() }, query: { _ in query })
  }

  deinit {
    watcher?.cancel()
    consumer?.cancel()
  }

  public func refetch() {
    guard let watcher else {
      refetchPending = currentInput != nil
      return
    }
    Task { await watcher.refetch() }
  }

  private func observeInput() {
    let next = withObservationTracking {
      input()
    } onChange: { [weak self] in
      Task { @MainActor [weak self] in self?.observeInput() }
    }
    apply(next)
  }

  private func apply(_ next: Input?) {
    guard next != currentInput else { return }
    currentInput = next
    stop()
    if !keepsDataOnInputChange || next == nil {
      data = nil
    }
    error = nil
    isSettled = false
    if let next {
      start(query(next))
    }
  }

  private func start(_ query: Query) {
    generation += 1
    let generation = generation
    let (stream, continuation) = AsyncStream.makeStream(
      of: Result<Query.Data, any Error>.self)
    consumer = Task { [weak self] in
      for await result in stream {
        guard let self, self.generation == generation else { return }
        receive(result)
      }
    }
    let client = client
    Task { [weak self] in
      let watcher = await client.watch(query) { continuation.yield($0) }
      guard let self, self.generation == generation else {
        watcher.cancel()
        return
      }
      self.watcher = watcher
      if refetchPending {
        refetchPending = false
        refetch()
      }
    }
  }

  private func stop() {
    generation += 1
    watcher?.cancel()
    watcher = nil
    consumer?.cancel()
    consumer = nil
    refetchPending = false
  }

  private func receive(_ result: Result<Query.Data, any Error>) {
    switch result {
    case .success(let data):
      self.data = data
      error = nil
      isSettled = true
    case .failure(let error):
      fail(error)
    }
  }

  private func fail(_ error: any Error) {
    self.error = error
    isSettled = true
  }
}
