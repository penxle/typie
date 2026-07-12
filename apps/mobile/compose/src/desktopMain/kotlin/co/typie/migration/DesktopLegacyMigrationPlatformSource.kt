package co.typie.migration

class DesktopLegacyMigrationPlatformSource : LegacyMigrationPlatformSource {
  override suspend fun load(): LegacyEncryptedHiveBoxSource? = null
}
