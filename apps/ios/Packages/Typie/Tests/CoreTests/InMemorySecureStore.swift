import Foundation

@testable import Core

final class InMemorySecureStore: SecureStore, @unchecked Sendable {
  private let lock = NSLock()
  private var items: [String: Data] = [:]

  init() {}

  func data(for key: String) throws -> Data? {
    lock.withLock { items[key] }
  }

  func set(_ data: Data?, for key: String) throws {
    lock.withLock { items[key] = data }
  }
}
