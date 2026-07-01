package co.typie.platform

import co.typie.editor.ffi.EditorHost
import co.typie.editor.ffi.JnaEditorHost
import co.typie.migration.DesktopLegacyMigrationPlatformSource
import co.typie.migration.LegacyMigrationPlatformSource
import co.typie.storage.DiskCache
import co.typie.storage.diskCache
import eu.anifantakis.lib.ksafe.KSafe
import eu.anifantakis.lib.ksafe.KSafeMemoryPolicy

actual object PlatformModule {
  actual val platform: Platform = Platform.Desktop
  actual val ksafePrefs: KSafe =
    KSafe(fileName = "prefs", memoryPolicy = KSafeMemoryPolicy.PLAIN_TEXT)
  actual val ksafeState: KSafe =
    KSafe(fileName = "state", memoryPolicy = KSafeMemoryPolicy.PLAIN_TEXT)
  actual val ksafeVault: KSafe = KSafe(fileName = "vault")
  actual val clipboard: Clipboard = DesktopClipboard()
  actual val deviceInfo: DeviceInfo = DesktopDeviceInfo()
  actual val fileSystem: FileSystem = DesktopFileSystem()
  actual val legacyMigrationPlatformSource: LegacyMigrationPlatformSource =
    DesktopLegacyMigrationPlatformSource()
  actual val purchaseService: PurchaseService = DesktopPurchaseService()
  actual val share: Share = DesktopShare()
  actual val editorHost: EditorHost = run {
    val icuData = JnaEditorHost::class.java.classLoader.getResourceAsStream("icu.zst")!!.readBytes()
    JnaEditorHost.create(icuData)
  }
  actual val diskCache: DiskCache = diskCache()
}
