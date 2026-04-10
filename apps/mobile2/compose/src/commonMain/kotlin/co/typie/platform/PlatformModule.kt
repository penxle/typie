package co.typie.platform

import co.typie.cache.DiskCache
import co.typie.editor.ffi.EditorHost
import co.typie.migration.LegacyMigrationPlatformSource
import eu.anifantakis.lib.ksafe.KSafe

enum class Platform {
  Android,
  iOS,
  Desktop,
}

expect object PlatformModule {
  val platform: Platform
  val ksafePrefs: KSafe
  val ksafeVault: KSafe
  val clipboard: Clipboard
  val deviceInfo: DeviceInfo
  val fileSystem: FileSystem
  val legacyMigrationPlatformSource: LegacyMigrationPlatformSource
  val purchaseService: PurchaseService
  val share: Share
  val editorHost: EditorHost
  val diskCache: DiskCache
}
