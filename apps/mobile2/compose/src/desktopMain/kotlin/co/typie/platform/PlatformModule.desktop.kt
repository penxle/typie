package co.typie.platform

import co.typie.cache.DiskCache
import co.typie.cache.diskCache
import co.typie.editor.ffi.BackendKind
import co.typie.editor.ffi.EditorHost
import co.typie.editor.ffi.JnaEditorHost
import co.typie.migration.DesktopLegacyMigrationPlatformSource
import co.typie.migration.LegacyMigrationPlatformSource
import eu.anifantakis.lib.ksafe.KSafe
import eu.anifantakis.lib.ksafe.KSafeMemoryPolicy
import kotlinx.coroutines.runBlocking

actual object PlatformModule {
  actual val platform: Platform = Platform.Desktop
  actual val ksafePrefs: KSafe =
    KSafe(fileName = "prefs", memoryPolicy = KSafeMemoryPolicy.PLAIN_TEXT)
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
    runBlocking { JnaEditorHost.create(BackendKind.Cpu, icuData) }
  }
  actual val diskCache: DiskCache = diskCache()
}
