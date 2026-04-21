import Security
import XCTest
@testable import Bridge

final class LegacyMigrationBridgeTests: XCTestCase {
  func testMissingKeychainValueReturnsNilWithoutCrashing() {
    let bridge = LegacyMigrationBridge()

    let value = bridge.readValue(
      key: LegacyMigrationBridge.hiveEncryptionKeyName,
      copyMatching: { _, _ in errSecItemNotFound }
    )

    XCTAssertNil(value)
  }

  func testHiveEncryptionKeyQueryUsesExpectedFlutterDefaults() {
    let query = LegacyMigrationBridge.makeQuery(
      key: LegacyMigrationBridge.hiveEncryptionKeyName,
      service: LegacyMigrationBridge.defaultServiceName
    )

    XCTAssertEqual(query[kSecClass] as? String, kSecClassGenericPassword as String)
    XCTAssertEqual(query[kSecAttrAccount] as? String, "hive_encryption_key")
    XCTAssertEqual(query[kSecAttrService] as? String, "flutter_secure_storage_service")
    XCTAssertEqual(query[kSecReturnData] as? Bool, true)
    XCTAssertEqual(query[kSecMatchLimit] as? String, kSecMatchLimitOne as String)
  }
}
