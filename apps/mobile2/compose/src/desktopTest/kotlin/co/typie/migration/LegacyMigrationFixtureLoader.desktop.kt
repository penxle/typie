package co.typie.migration

import java.security.MessageDigest
import java.util.zip.CRC32
import javax.crypto.Cipher
import javax.crypto.spec.IvParameterSpec
import javax.crypto.spec.SecretKeySpec

actual fun loadLegacyMigrationFixture(name: String): ByteArray {
  val path = "legacy_migration/$name"
  return checkNotNull(object {}.javaClass.classLoader.getResourceAsStream(path)) {
    "Missing legacy migration fixture: $path"
  }.use { it.readBytes() }
}

actual fun decryptLegacyHiveAesPayload(payload: ByteArray, key: ByteArray): ByteArray {
  require(payload.size >= 16) { "Encrypted Hive payload must contain an IV." }

  val cipher = Cipher.getInstance("AES/CBC/PKCS5Padding")
  cipher.init(
    Cipher.DECRYPT_MODE,
    SecretKeySpec(key, "AES"),
    IvParameterSpec(payload.copyOfRange(0, 16)),
  )

  return cipher.doFinal(payload.copyOfRange(16, payload.size))
}

actual fun calculateLegacyHiveKeyCrc(key: ByteArray): Long {
  val digest = MessageDigest.getInstance("SHA-256").digest(key)
  val crc32 = CRC32()
  crc32.update(digest)
  return crc32.value
}
