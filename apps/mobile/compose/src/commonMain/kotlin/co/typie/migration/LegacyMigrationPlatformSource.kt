package co.typie.migration

fun interface LegacyAuthPayloadDecryptor {
  fun decrypt(payload: ByteArray): ByteArray
}

data class LegacyEncryptedHiveBoxSource(
  val bytes: ByteArray,
  val keyCrc: Long,
  val decryptor: LegacyAuthPayloadDecryptor,
)

interface LegacyMigrationPlatformSource {
  suspend fun load(): LegacyEncryptedHiveBoxSource?
}
