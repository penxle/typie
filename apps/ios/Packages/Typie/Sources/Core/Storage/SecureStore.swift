import Foundation
import Security

public enum SecureStoreError: Error, Equatable {
  case unhandled(OSStatus)
}

public protocol SecureStore: Sendable {
  func data(for key: String) throws -> Data?
  func set(_ data: Data?, for key: String) throws
}

private let authTokensKey = "auth_tokens"

extension SecureStore {
  public func authTokens() throws -> AuthTokens? {
    guard let data = try data(for: authTokensKey) else { return nil }
    return try? JSONDecoder().decode(AuthTokens.self, from: data)
  }

  public func setAuthTokens(_ tokens: AuthTokens?) throws {
    guard let tokens else {
      try set(nil, for: authTokensKey)
      return
    }
    try set(JSONEncoder().encode(tokens), for: authTokensKey)
  }
}

public struct KeychainStore: SecureStore {
  private let service: String

  public init(service: String = "co.typie.vault") {
    self.service = service
  }

  public func data(for key: String) throws -> Data? {
    var query = query(for: key)
    query[kSecReturnData as String] = true
    query[kSecMatchLimit as String] = kSecMatchLimitOne

    var item: CFTypeRef?
    let status = SecItemCopyMatching(query as CFDictionary, &item)
    switch status {
    case errSecSuccess: return item as? Data
    case errSecItemNotFound: return nil
    default: throw SecureStoreError.unhandled(status)
    }
  }

  public func set(_ data: Data?, for key: String) throws {
    guard let data else {
      let status = SecItemDelete(query(for: key) as CFDictionary)
      guard status == errSecSuccess || status == errSecItemNotFound else {
        throw SecureStoreError.unhandled(status)
      }
      return
    }

    let attributes: [String: Any] = [
      kSecValueData as String: data,
      kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly,
    ]
    let status = SecItemUpdate(query(for: key) as CFDictionary, attributes as CFDictionary)
    switch status {
    case errSecSuccess: return
    case errSecItemNotFound:
      let item = query(for: key).merging(attributes) { _, new in new }
      let addStatus = SecItemAdd(item as CFDictionary, nil)
      if addStatus == errSecDuplicateItem {
        let retryStatus = SecItemUpdate(
          query(for: key) as CFDictionary, attributes as CFDictionary)
        guard retryStatus == errSecSuccess else { throw SecureStoreError.unhandled(retryStatus) }
        return
      }
      guard addStatus == errSecSuccess else { throw SecureStoreError.unhandled(addStatus) }
    default: throw SecureStoreError.unhandled(status)
    }
  }

  private func query(for key: String) -> [String: Any] {
    [
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrService as String: service,
      kSecAttrAccount as String: key,
    ]
  }
}

public final class InMemorySecureStore: SecureStore, @unchecked Sendable {
  private let lock = NSLock()
  private var items: [String: Data] = [:]

  public init() {}

  public func data(for key: String) throws -> Data? {
    lock.withLock { items[key] }
  }

  public func set(_ data: Data?, for key: String) throws {
    lock.withLock { items[key] = data }
  }
}
