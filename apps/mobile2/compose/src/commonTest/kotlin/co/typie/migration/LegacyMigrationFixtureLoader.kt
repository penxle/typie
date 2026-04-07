package co.typie.migration

expect fun loadLegacyMigrationFixture(name: String): ByteArray

expect fun decryptLegacyHiveAesPayload(payload: ByteArray, key: ByteArray): ByteArray

expect fun calculateLegacyHiveKeyCrc(key: ByteArray): Long
