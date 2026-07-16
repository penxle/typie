package co.typie.platform

import androidx.compose.ui.input.pointer.PointerType
import co.typie.editor.ffi.EditorHost
import co.typie.migration.LegacyMigrationPlatformSource
import co.typie.storage.DiskCache
import eu.anifantakis.lib.ksafe.KSafe

enum class Platform {
  Android,
  iOS,
  Desktop,
}

expect object PlatformModule {
  val platform: Platform
  val ksafePrefs: KSafe
  val ksafeState: KSafe
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

internal expect fun PointerType.isTouchDragPointer(): Boolean
