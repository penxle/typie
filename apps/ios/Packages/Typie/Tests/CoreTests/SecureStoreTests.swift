import Foundation
import Testing

@testable import Core

@Suite struct SecureStoreTests {
  private let key = "auth_tokens"

  private func tokens(_ suffix: String) -> AuthTokens {
    AuthTokens(
      sessionToken: "session-\(suffix)",
      accessToken: "access-\(suffix)",
      userId: "user-\(suffix)"
    )
  }

  @Test func inMemoryStoreRoundTripsData() throws {
    let store = InMemorySecureStore()
    #expect(try store.data(for: key) == nil)
    try store.set(Data([0x01, 0x02, 0x03]), for: key)
    #expect(try store.data(for: key) == Data([0x01, 0x02, 0x03]))
    try store.set(Data([0x04]), for: key)
    #expect(try store.data(for: key) == Data([0x04]))
  }

  @Test func inMemoryStoreRemovesValueOnNil() throws {
    let store = InMemorySecureStore()
    try store.set(Data([0x01]), for: key)
    try store.set(nil, for: key)
    #expect(try store.data(for: key) == nil)
    try store.set(nil, for: key)
    #expect(try store.data(for: key) == nil)
  }

  @Test func inMemoryStoreKeepsKeysSeparate() throws {
    let store = InMemorySecureStore()
    try store.set(Data([0x01]), for: "a")
    try store.set(Data([0x02]), for: "b")
    #expect(try store.data(for: "a") == Data([0x01]))
    #expect(try store.data(for: "b") == Data([0x02]))
  }

  @Test func authTokensRoundTripThroughStore() throws {
    let store = InMemorySecureStore()
    #expect(try store.authTokens() == nil)
    try store.setAuthTokens(tokens("1"))
    #expect(try store.authTokens() == tokens("1"))
    try store.setAuthTokens(tokens("2"))
    #expect(try store.authTokens() == tokens("2"))
    try store.setAuthTokens(nil)
    #expect(try store.authTokens() == nil)
  }

  @Test func undecodableAuthTokensReadAsNil() throws {
    let store = InMemorySecureStore()
    try store.set(Data("not json".utf8), for: "auth_tokens")
    #expect(try store.authTokens() == nil)
  }

  @Test func authTokensAreStoredAsOneJSONRecord() throws {
    let store = InMemorySecureStore()
    try store.setAuthTokens(tokens("1"))
    let data = try #require(try store.data(for: key))
    let object = try #require(JSONSerialization.jsonObject(with: data) as? [String: String])
    #expect(object.keys.sorted() == ["accessToken", "sessionToken", "userId"])
  }

  @Test func keychainStoreAddsUpdatesAndDeletes() throws {
    let store = KeychainStore(service: "co.typie.tests.\(UUID().uuidString)")
    defer { try? store.set(nil, for: key) }

    try withKnownIssueIfKeychainIsUnavailable {
      let empty = try store.data(for: key)
      #expect(empty == nil)

      try store.setAuthTokens(tokens("1"))
      let added = try store.authTokens()
      #expect(added == tokens("1"))

      try store.setAuthTokens(tokens("2"))
      let updated = try store.authTokens()
      #expect(updated == tokens("2"))

      try store.set(nil, for: key)
      let removed = try store.data(for: key)
      #expect(removed == nil)

      try store.set(nil, for: key)
    }
  }

  @Test func keychainStoresAreIsolatedByService() throws {
    let one = KeychainStore(service: "co.typie.tests.\(UUID().uuidString)")
    let other = KeychainStore(service: "co.typie.tests.\(UUID().uuidString)")
    defer {
      try? one.set(nil, for: key)
      try? other.set(nil, for: key)
    }

    try withKnownIssueIfKeychainIsUnavailable {
      try one.set(Data([0x01]), for: key)
      let missing = try other.data(for: key)
      #expect(missing == nil)
      let stored = try one.data(for: key)
      #expect(stored == Data([0x01]))
    }
  }

  private func withKnownIssueIfKeychainIsUnavailable(_ body: () throws -> Void) throws {
    do {
      try body()
    } catch let error as SecureStoreError {
      guard case let .unhandled(status) = error, Self.unavailableStatuses.contains(status) else {
        throw error
      }
      withKnownIssue("host keychain refused access: OSStatus \(status)") { throw error }
    }
  }

  private static let unavailableStatuses: Set<OSStatus> = [
    errSecMissingEntitlement, errSecInteractionNotAllowed,
  ]
}
