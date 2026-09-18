import Testing

@testable import Core

@Suite struct AsyncLockTests {
  @Test func runsCriticalSectionsOneAtATime() async {
    let lock = AsyncLock()
    let probe = ConcurrencyProbe()

    await withTaskGroup(of: Void.self) { group in
      for _ in 0..<100 {
        group.addTask {
          try? await lock.withLock {
            probe.enter()
            await Task.yield()
            probe.exit()
          }
        }
      }
    }

    #expect(probe.maxConcurrent == 1)
    #expect(probe.finished == 100)
  }

  @Test func grantsWaitersInArrivalOrder() async throws {
    let lock = AsyncLock()
    let gate = TestGate()
    let recorder = CallRecorder()

    let holder = Task {
      try await lock.withLock {
        recorder.record("holder")
        await gate.wait()
      }
    }
    try await waitUntil { recorder.calls == ["holder"] }

    var waiters: [Task<Void, any Error>] = []
    for index in 0..<10 {
      waiters.append(
        Task {
          try await lock.withLock { recorder.record("w\(index)") }
        })
      try await waitUntil { lock.waitingCount == index + 1 }
    }

    await gate.open()
    try await holder.value
    for waiter in waiters { try await waiter.value }

    #expect(recorder.calls == ["holder"] + (0..<10).map { "w\($0)" })
  }

  @Test func cancelledWaiterSkipsCriticalSectionAndUnblocksTheNextOne() async throws {
    let lock = AsyncLock()
    let gate = TestGate()
    let recorder = CallRecorder()

    let holder = Task {
      try await lock.withLock {
        recorder.record("holder")
        await gate.wait()
      }
    }
    try await waitUntil { recorder.calls == ["holder"] }

    let first = Task { try await lock.withLock { recorder.record("first") } }
    try await waitUntil { lock.waitingCount == 1 }
    let second = Task { try await lock.withLock { recorder.record("second") } }
    try await waitUntil { lock.waitingCount == 2 }

    first.cancel()
    try await waitUntil { lock.waitingCount == 1 }

    await gate.open()
    try await holder.value
    try await second.value
    await #expect(throws: CancellationError.self) { try await first.value }

    #expect(recorder.calls == ["holder", "second"])
  }

  @Test func cancelledTaskNeverEntersAndLeavesTheLockUsable() async throws {
    let lock = AsyncLock()
    let recorder = CallRecorder()

    let gate = TestGate()
    let cancelled = Task {
      await gate.wait()
      try await lock.withLock { recorder.record("cancelled") }
    }
    cancelled.cancel()
    await gate.open()
    await #expect(throws: CancellationError.self) { try await cancelled.value }

    try await lock.withLock { recorder.record("after") }

    #expect(recorder.calls == ["after"])
  }
}
