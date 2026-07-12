@file:OptIn(BetaInteropApi::class)

package co.typie.migration

import kotlinx.cinterop.BetaInteropApi
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import platform.Foundation.NSData
import platform.Foundation.NSDocumentDirectory
import platform.Foundation.NSSearchPathForDirectoriesInDomains
import platform.Foundation.NSUserDomainMask
import platform.Foundation.create

internal class IOSLegacyMigrationPlatformSource(
  private val bridge: LegacyMigrationBridge = LegacyMigrationBridge()
) : LegacyMigrationPlatformSource {
  override suspend fun load(): LegacyEncryptedHiveBoxSource? =
    withContext(Dispatchers.Default) {
      val documentsDirectory =
        NSSearchPathForDirectoriesInDomains(NSDocumentDirectory, NSUserDomainMask, true)
          .firstOrNull() as? String ?: return@withContext null

      val authBoxBytes = loadBox(documentsDirectory, "auth_box") ?: return@withContext null
      val base64HiveKey = bridge.readHiveEncryptionKey() ?: return@withContext null
      val keyCrc = bridge.calculateLegacyHiveKeyCrc(base64HiveKey) ?: return@withContext null

      LegacyEncryptedHiveBoxSource(
        bytes = authBoxBytes,
        keyCrc = keyCrc,
        decryptor =
          LegacyAuthPayloadDecryptor { payload ->
            bridge.decryptLegacyHivePayload(payload, base64HiveKey)
              ?: error("Failed to decrypt legacy Hive auth payload.")
          },
      )
    }

  private fun loadBox(documentsDirectory: String, name: String): ByteArray? {
    return NSData.create(contentsOfFile = "$documentsDirectory/$name.hive")?.toByteArray()
  }
}
