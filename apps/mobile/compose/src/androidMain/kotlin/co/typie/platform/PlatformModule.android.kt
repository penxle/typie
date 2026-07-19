package co.typie.platform

import android.annotation.SuppressLint
import android.content.Context
import androidx.compose.ui.input.pointer.PointerEvent
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.PointerType
import co.typie.editor.ffi.EditorHost
import co.typie.editor.ffi.JnaEditorHost
import co.typie.migration.AndroidLegacyMigrationPlatformSource
import co.typie.migration.LegacyMigrationPlatformSource
import co.typie.storage.DiskCache
import co.typie.storage.diskCache
import eu.anifantakis.lib.ksafe.KSafe
import eu.anifantakis.lib.ksafe.KSafeMemoryPolicy

@SuppressLint("StaticFieldLeak")
actual object PlatformModule {
  lateinit var context: Context

  actual val platform: Platform = Platform.Android
  actual val ksafePrefs: KSafe by lazy {
    KSafe(context = context, fileName = "prefs", memoryPolicy = KSafeMemoryPolicy.PLAIN_TEXT)
  }
  actual val ksafeState: KSafe by lazy {
    KSafe(context = context, fileName = "state", memoryPolicy = KSafeMemoryPolicy.PLAIN_TEXT)
  }
  actual val ksafeVault: KSafe by lazy { KSafe(context = context, fileName = "vault") }
  actual val clipboard: Clipboard by lazy { AndroidClipboard(context) }
  actual val deviceInfo: DeviceInfo by lazy { AndroidDeviceInfo(context) }
  actual val fileSystem: FileSystem by lazy { AndroidFileSystem(context) }
  actual val legacyMigrationPlatformSource: LegacyMigrationPlatformSource by lazy {
    AndroidLegacyMigrationPlatformSource(context)
  }
  actual val purchaseService: PurchaseService by lazy { AndroidPurchaseService(context) }
  actual val share: Share by lazy { AndroidShare(context) }
  actual val editorHost: EditorHost by lazy {
    val icuData = context.assets.open("icu.zst").readBytes()
    JnaEditorHost.create(icuData)
  }
  actual val diskCache: DiskCache by lazy { diskCache() }
}

internal actual fun PointerType.isTouchDragPointer(): Boolean = this == PointerType.Touch

internal actual fun PointerEvent.isDirectMousePress(): Boolean = type == PointerEventType.Press
