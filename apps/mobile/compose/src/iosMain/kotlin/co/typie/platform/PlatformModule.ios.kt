@file:OptIn(ExperimentalForeignApi::class, BetaInteropApi::class)

package co.typie.platform

import androidx.compose.ui.input.pointer.PointerType
import co.typie.editor.ffi.EditorHost
import co.typie.editor.ffi.IosEditorHost
import co.typie.migration.IOSLegacyMigrationPlatformSource
import co.typie.migration.LegacyMigrationPlatformSource
import co.typie.storage.DiskCache
import co.typie.storage.diskCache
import eu.anifantakis.lib.ksafe.KSafe
import eu.anifantakis.lib.ksafe.KSafeMemoryPolicy
import kotlinx.cinterop.BetaInteropApi
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.addressOf
import kotlinx.cinterop.usePinned
import platform.Foundation.NSBundle
import platform.Foundation.NSData
import platform.Foundation.create
import platform.posix.memcpy

actual object PlatformModule {
  actual val platform: Platform = Platform.iOS
  actual val ksafePrefs: KSafe =
    KSafe(fileName = "prefs", memoryPolicy = KSafeMemoryPolicy.PLAIN_TEXT)
  actual val ksafeState: KSafe =
    KSafe(fileName = "state", memoryPolicy = KSafeMemoryPolicy.PLAIN_TEXT)
  actual val ksafeVault: KSafe = KSafe(fileName = "vault")
  actual val clipboard: Clipboard = IOSClipboard()
  actual val deviceInfo: DeviceInfo = IOSDeviceInfo()
  actual val fileSystem: FileSystem = IOSFileSystem()
  actual val legacyMigrationPlatformSource: LegacyMigrationPlatformSource =
    IOSLegacyMigrationPlatformSource()
  actual val purchaseService: PurchaseService = IOSPurchaseService()
  actual val share: Share = IOSShare()
  actual val editorHost: EditorHost = run {
    val path = NSBundle.mainBundle.pathForResource("icu", "zst")!!
    val nsData = NSData.create(contentsOfFile = path)!!
    val icuData =
      ByteArray(nsData.length.toInt()).apply {
        usePinned { memcpy(it.addressOf(0), nsData.bytes, nsData.length) }
      }
    IosEditorHost.create(icuData)
  }
  actual val diskCache: DiskCache = diskCache()
}

internal actual fun PointerType.isTouchDragPointer(): Boolean = this == PointerType.Touch
