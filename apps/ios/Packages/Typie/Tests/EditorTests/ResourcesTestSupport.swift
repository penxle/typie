import EditorFFI

@testable import Editor

struct FakeFailure: Error {}

@MainActor
final class ReceiptLog {
  var count = 0
}

@MainActor
final class FakeReceiver: ResourceReceiver {
  private(set) var received: [RResourceUpdate] = []
  var failing = false
  private let log: ReceiptLog?

  init(log: ReceiptLog? = nil) {
    self.log = log
  }

  func receiveResourceUpdate(_ update: RResourceUpdate) throws {
    if failing { throw FakeFailure() }
    received.append(update)
    log?.count += 1
  }
}

@MainActor
final class RawReceiver: ResourceReceiver {
  let raw: REditor

  init(_ raw: REditor) {
    self.raw = raw
  }

  func receiveResourceUpdate(_ update: RResourceUpdate) throws {
    try raw.receiveResourceUpdate(update: update)
  }
}
