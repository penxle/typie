import CommonCrypto
import Foundation
import Security

typealias LegacyMigrationCopyMatchingHandler = (
  _ query: CFDictionary,
  _ result: UnsafeMutablePointer<CFTypeRef?>?
) -> OSStatus

@objcMembers public final class LegacyMigrationBridge: NSObject {
  static let hiveEncryptionKeyName = "hive_encryption_key"
  static let defaultServiceName = "flutter_secure_storage_service"

  public func readHiveEncryptionKey() -> String? {
    readValue(key: Self.hiveEncryptionKeyName)
  }

  func readValue(
    key: String,
    service: String = LegacyMigrationBridge.defaultServiceName,
    copyMatching: LegacyMigrationCopyMatchingHandler = SecItemCopyMatching
  ) -> String? {
    var result: CFTypeRef?
    let status = copyMatching(Self.makeQuery(key: key, service: service) as CFDictionary, &result)

    guard status == errSecSuccess else {
      return nil
    }

    guard let data = result as? Data else {
      return nil
    }

    return String(data: data, encoding: .utf8)
  }

  static func makeQuery(
    key: String,
    service: String = LegacyMigrationBridge.defaultServiceName
  ) -> [CFString: Any] {
    [
      kSecClass: kSecClassGenericPassword,
      kSecAttrAccount: key,
      kSecAttrService: service,
      kSecReturnData: true,
      kSecMatchLimit: kSecMatchLimitOne,
    ]
  }

  public func calculateLegacyHiveKeyCrc(base64EncodedKey: String) -> NSNumber? {
    guard let keyData = Data(base64Encoded: base64EncodedKey) else {
      return nil
    }

    let digest = sha256(of: keyData)
    let crc = digest.withUnsafeBytes { bytes -> UInt32 in
      guard let baseAddress = bytes.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
        return 0
      }
      return Self.calculateLegacyCRC32(
        bytes: UnsafeBufferPointer(start: baseAddress, count: digest.count)
      )
    }

    return NSNumber(value: Int64(crc))
  }

  public func decryptLegacyHivePayload(
    payload: Data,
    base64EncodedKey: String
  ) -> Data? {
    guard payload.count >= kCCBlockSizeAES128 else {
      return nil
    }

    guard let keyData = Data(base64Encoded: base64EncodedKey) else {
      return nil
    }

    let iv = payload.prefix(kCCBlockSizeAES128)
    let cipherText = payload.dropFirst(kCCBlockSizeAES128)
    var plaintext = Data(count: cipherText.count + kCCBlockSizeAES128)
    let plaintextCapacity = plaintext.count
    var plaintextLength = 0

    let status = plaintext.withUnsafeMutableBytes { plaintextBytes in
      cipherText.withUnsafeBytes { cipherTextBytes in
        iv.withUnsafeBytes { ivBytes in
          keyData.withUnsafeBytes { keyBytes in
            CCCrypt(
              CCOperation(kCCDecrypt),
              CCAlgorithm(kCCAlgorithmAES),
              CCOptions(kCCOptionPKCS7Padding),
              keyBytes.baseAddress,
              keyData.count,
              ivBytes.baseAddress,
              cipherTextBytes.baseAddress,
              cipherText.count,
              plaintextBytes.baseAddress,
              plaintextCapacity,
              &plaintextLength
            )
          }
        }
      }
    }

    guard status == kCCSuccess else {
      return nil
    }

    plaintext.count = plaintextLength
    return plaintext
  }

  private func sha256(of data: Data) -> Data {
    var digest = Data(count: Int(CC_SHA256_DIGEST_LENGTH))
    _ = digest.withUnsafeMutableBytes { digestBytes in
      data.withUnsafeBytes { dataBytes in
        CC_SHA256(
          dataBytes.baseAddress,
          CC_LONG(data.count),
          digestBytes.bindMemory(to: UInt8.self).baseAddress
        )
      }
    }
    return digest
  }

  private static func calculateLegacyCRC32(
    bytes: UnsafeBufferPointer<UInt8>,
    seed: UInt32 = 0
  ) -> UInt32 {
    var crc = seed ^ UInt32.max

    for byte in bytes {
      let tableIndex = Int((crc ^ UInt32(byte)) & 0xFF)
      crc = legacyCRC32Table[tableIndex] ^ (crc >> 8)
    }

    return crc ^ UInt32.max
  }

  private static let legacyCRC32Table: [UInt32] = (0..<256).map { index in
    var value = UInt32(index)
    for _ in 0..<8 {
      if (value & 1) != 0 {
        value = 0xEDB8_8320 ^ (value >> 1)
      } else {
        value >>= 1
      }
    }
    return value
  }
}
