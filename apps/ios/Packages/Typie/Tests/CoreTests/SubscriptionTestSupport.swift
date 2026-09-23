import Foundation

@testable import Core

final class TicketSource: @unchecked Sendable {
  private let lock = NSLock()
  private let failures: Int
  private let gate: TestGate?
  private var requested = 0
  private var issued = 0

  init(failures: Int = 0, gate: TestGate? = nil) {
    self.failures = failures
    self.gate = gate
  }

  var requests: Int { lock.withLock { requested } }
  var count: Int { lock.withLock { issued } }

  func next() async throws -> String {
    lock.withLock { requested += 1 }
    await gate?.wait()
    let number = lock.withLock { () -> Int in
      issued += 1
      return issued
    }
    if number <= failures { throw HTTPError.network("offline") }
    return "ticket-\(number)"
  }
}

final class Counter: @unchecked Sendable {
  private let lock = NSLock()
  private var current = 0

  func increment() { lock.withLock { current += 1 } }

  var value: Int { lock.withLock { current } }
}
