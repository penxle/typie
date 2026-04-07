@file:OptIn(kotlinx.cinterop.ExperimentalForeignApi::class)

package co.typie.migration

import kotlinx.cinterop.addressOf
import kotlinx.cinterop.usePinned
import platform.Foundation.NSData
import platform.Foundation.create
import platform.posix.memcpy
import swiftPMImport.co.typie.compose.LegacyMigrationBridge as SwiftLegacyMigrationBridge

internal class LegacyMigrationBridge {
  private val bridge = SwiftLegacyMigrationBridge()

  fun readHiveEncryptionKey(): String? = bridge.readHiveEncryptionKey()

  fun calculateLegacyHiveKeyCrc(base64EncodedKey: String): Long? {
    return bridge.calculateLegacyHiveKeyCrcWithBase64EncodedKey(base64EncodedKey)?.longLongValue
  }

  fun decryptLegacyHivePayload(
    payload: ByteArray,
    base64EncodedKey: String,
  ): ByteArray? {
    return bridge
      .decryptLegacyHivePayloadWithPayload(
        payload = payload.toNSData(),
        base64EncodedKey = base64EncodedKey,
      )
      ?.toByteArray()
  }
}

internal fun ByteArray.toNSData(): NSData {
  return usePinned { pinned ->
    NSData.create(
      bytes = pinned.addressOf(0),
      length = size.toULong(),
    )
  }
}

internal fun NSData.toByteArray(): ByteArray {
  val byteArray = ByteArray(length.toInt())
  byteArray.usePinned { pinned ->
    memcpy(pinned.addressOf(0), bytes, length)
  }
  return byteArray
}
