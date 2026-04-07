package co.typie.migration

fun interface LegacyAuthPayloadDecryptor {
  fun decrypt(payload: ByteArray): ByteArray
}

data class LegacyEncryptedHiveBoxSource(
  val bytes: ByteArray,
  val keyCrc: Long,
  val decryptor: LegacyAuthPayloadDecryptor,
)

data class LegacyMigrationSource(
  val authBox: LegacyEncryptedHiveBoxSource? = null,
  val preferenceBox: ByteArray? = null,
  val themeBox: ByteArray? = null,
)

interface LegacyMigrationPlatformSource {
  suspend fun load(): LegacyMigrationSource?
}

class NoOpLegacyMigrationPlatformSource : LegacyMigrationPlatformSource {
  override suspend fun load(): LegacyMigrationSource? = null
}
