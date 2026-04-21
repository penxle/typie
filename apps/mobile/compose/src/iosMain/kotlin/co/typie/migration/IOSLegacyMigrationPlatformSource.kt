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
  override suspend fun load(): LegacyMigrationSource? =
    withContext(Dispatchers.Default) {
      val documentsDirectory =
        NSSearchPathForDirectoriesInDomains(NSDocumentDirectory, NSUserDomainMask, true)
          .firstOrNull() as? String ?: return@withContext null

      val authBoxBytes = loadBox(documentsDirectory, "auth_box")
      val preferenceBoxBytes = loadBox(documentsDirectory, "preference_box")
      val themeBoxBytes = loadBox(documentsDirectory, "theme_box")
      val base64HiveKey = bridge.readHiveEncryptionKey()

      val authBoxSource =
        if (authBoxBytes != null && base64HiveKey != null) {
          val keyCrc =
            bridge.calculateLegacyHiveKeyCrc(base64HiveKey)
              ?: return@withContext nullSource(
                preferenceBox = preferenceBoxBytes,
                themeBox = themeBoxBytes,
              )

          LegacyEncryptedHiveBoxSource(
            bytes = authBoxBytes,
            keyCrc = keyCrc,
            decryptor =
              LegacyAuthPayloadDecryptor { payload ->
                bridge.decryptLegacyHivePayload(payload, base64HiveKey)
                  ?: error("Failed to decrypt legacy Hive auth payload.")
              },
          )
        } else {
          null
        }

      LegacyMigrationSource(
          authBox = authBoxSource,
          preferenceBox = preferenceBoxBytes,
          themeBox = themeBoxBytes,
        )
        .takeIf { it.authBox != null || it.preferenceBox != null || it.themeBox != null }
    }

  private fun loadBox(documentsDirectory: String, name: String): ByteArray? {
    return NSData.create(contentsOfFile = "$documentsDirectory/$name.hive")?.toByteArray()
  }

  private fun nullSource(preferenceBox: ByteArray?, themeBox: ByteArray?): LegacyMigrationSource? {
    return LegacyMigrationSource(authBox = null, preferenceBox = preferenceBox, themeBox = themeBox)
      .takeIf { it.preferenceBox != null || it.themeBox != null }
  }
}
