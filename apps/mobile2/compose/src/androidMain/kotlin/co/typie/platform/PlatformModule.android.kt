package co.typie.platform

import android.content.Context
import co.typie.cache.DiskCache
import co.typie.cache.diskCache
import co.typie.editor.ffi.BackendKind
import co.typie.editor.ffi.EditorHost
import co.typie.editor.ffi.JnaEditorHost
import co.typie.migration.AndroidLegacyMigrationPlatformSource
import co.typie.migration.LegacyMigrationPlatformSource
import eu.anifantakis.lib.ksafe.KSafe
import eu.anifantakis.lib.ksafe.KSafeMemoryPolicy
import kotlinx.coroutines.runBlocking

actual object PlatformModule {
  lateinit var context: Context

  actual val platform: Platform get() = Platform.Android
  actual val ksafePrefs: KSafe get() = KSafe(context = context, fileName = "prefs", memoryPolicy = KSafeMemoryPolicy.PLAIN_TEXT)
  actual val ksafeVault: KSafe get() = KSafe(context = context, fileName = "vault")
  actual val clipboard: Clipboard get() = AndroidClipboard(context)
  actual val deviceInfo: DeviceInfo get() = AndroidDeviceInfo(context)
  actual val fileSystem: FileSystem get() = AndroidFileSystem(context)
  actual val legacyMigrationPlatformSource: LegacyMigrationPlatformSource get() = AndroidLegacyMigrationPlatformSource(context)
  actual val purchaseService: PurchaseService get() = AndroidPurchaseService(context)
  actual val share: Share get() = AndroidShare(context)
  actual val editorHost: EditorHost get() {
    val icuData = context.assets.open("icu.zst").readBytes()
    return runBlocking { JnaEditorHost.create(BackendKind.Gpu, icuData) }
  }
  actual val diskCache: DiskCache get() = diskCache()
}
