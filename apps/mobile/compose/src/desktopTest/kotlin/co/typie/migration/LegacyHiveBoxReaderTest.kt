package co.typie.migration

import kotlin.test.Test
import kotlin.test.assertEquals

class LegacyHiveBoxReaderTest {
  private val reader = LegacyHiveBoxReader

  @Test
  fun `readEncryptedBox decodes auth box session token`() {
    val encryptionKey =
      byteArrayOf(
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        10,
        11,
        12,
        13,
        14,
        15,
        16,
        17,
        18,
        19,
        20,
        21,
        22,
        23,
        24,
        25,
        26,
        27,
        28,
        29,
        30,
        31,
      )

    val values =
      reader.readEncryptedBox(
        bytes = loadLegacyMigrationFixture("auth_box.hive"),
        keyCrc = calculateLegacyHiveKeyCrc(encryptionKey),
        decrypt = { payload: ByteArray -> decryptLegacyHiveAesPayload(payload, encryptionKey) },
      )

    assertEquals(mapOf("session_token" to "fixture-session-token"), values)
  }
}
