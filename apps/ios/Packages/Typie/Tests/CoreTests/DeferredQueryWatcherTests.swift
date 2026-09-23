import Foundation
import Testing

@testable import Core

private final class RecordingWatcher: QueryWatcher, @unchecked Sendable {
  private let lock = NSLock()
  private var refetches = 0
  private var cancels = 0

  var refetchCount: Int { lock.withLock { refetches } }
  var cancelCount: Int { lock.withLock { cancels } }

  func refetch() async {
    lock.withLock { refetches += 1 }
  }

  func cancel() {
    lock.withLock { cancels += 1 }
  }
}

struct DeferredQueryWatcherTests {
  @Test func refetchBeforeAttachRunsOnceAfterAttach() async throws {
    let deferred = DeferredQueryWatcher()
    let watcher = RecordingWatcher()
    await deferred.refetch()
    await deferred.refetch()
    #expect(watcher.refetchCount == 0)
    deferred.attach(watcher)
    try await waitUntil { watcher.refetchCount == 1 }
    await deferred.refetch()
    #expect(watcher.refetchCount == 2)
  }

  @Test func cancelBeforeAttachCancelsOnAttach() {
    let deferred = DeferredQueryWatcher()
    let watcher = RecordingWatcher()
    deferred.cancel()
    #expect(watcher.cancelCount == 0)
    deferred.attach(watcher)
    #expect(watcher.cancelCount == 1)
  }

  @Test func cancelAfterAttachForwardsImmediately() {
    let deferred = DeferredQueryWatcher()
    let watcher = RecordingWatcher()
    deferred.attach(watcher)
    #expect(watcher.cancelCount == 0)
    deferred.cancel()
    #expect(watcher.cancelCount == 1)
  }

  @Test func cancelledWatcherDoesNotRunPendingRefetch() async throws {
    let deferred = DeferredQueryWatcher()
    let watcher = RecordingWatcher()
    await deferred.refetch()
    deferred.cancel()
    deferred.attach(watcher)
    try await Task.sleep(for: .milliseconds(20))
    #expect(watcher.cancelCount == 1)
    #expect(watcher.refetchCount == 0)
  }
}
