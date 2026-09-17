import Synchronization

final class AsyncLock: Sendable {
  private struct Waiter {
    let id: UInt64
    let continuation: CheckedContinuation<Void, any Error>
  }

  private struct State {
    var isLocked = false
    var queue: [Waiter] = []
    var cancelled: Set<UInt64> = []
    var nextID: UInt64 = 0
  }

  private enum Admission {
    case granted
    case cancelled
    case queued
  }

  private let state = Mutex(State())

  init() {}

  var waitingCount: Int {
    state.withLock { $0.queue.count }
  }

  func withLock<T: Sendable>(_ body: () async throws -> T) async throws -> T {
    try await acquire()
    defer { release() }
    return try await body()
  }

  private func acquire() async throws {
    let id = state.withLock { state -> UInt64 in
      state.nextID += 1
      return state.nextID
    }
    defer { state.withLock { _ = $0.cancelled.remove(id) } }

    try await withTaskCancellationHandler {
      try await withCheckedThrowingContinuation {
        (continuation: CheckedContinuation<Void, any Error>) in
        let admission = state.withLock { state -> Admission in
          if state.cancelled.remove(id) != nil { return .cancelled }
          if !state.isLocked && state.queue.isEmpty {
            state.isLocked = true
            return .granted
          }
          state.queue.append(Waiter(id: id, continuation: continuation))
          return .queued
        }

        switch admission {
        case .granted: continuation.resume()
        case .cancelled: continuation.resume(throwing: CancellationError())
        case .queued: break
        }
      }
    } onCancel: {
      let waiter = state.withLock { state -> Waiter? in
        guard let index = state.queue.firstIndex(where: { $0.id == id }) else {
          state.cancelled.insert(id)
          return nil
        }
        return state.queue.remove(at: index)
      }
      waiter?.continuation.resume(throwing: CancellationError())
    }
  }

  private func release() {
    let next = state.withLock { state -> Waiter? in
      guard !state.queue.isEmpty else {
        state.isLocked = false
        return nil
      }
      return state.queue.removeFirst()
    }
    next?.continuation.resume()
  }
}
