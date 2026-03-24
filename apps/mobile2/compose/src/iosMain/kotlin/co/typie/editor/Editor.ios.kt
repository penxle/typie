package co.typie.editor

import co.typie.di.PlatformContext
import kotlinx.cinterop.addressOf
import kotlinx.cinterop.allocArrayOf
import kotlinx.cinterop.memScoped
import kotlinx.cinterop.usePinned
import org.koin.core.annotation.Module
import org.koin.core.annotation.Single
import platform.Foundation.NSBundle
import platform.Foundation.NSData
import platform.Foundation.create
import platform.posix.memcpy
import swiftPMImport.co.typie.compose.NativeEditor
import swiftPMImport.co.typie.compose.NativeEditorEngine

@Module
actual class EditorModule {
  @Single
  actual fun editorEngine(ctx: PlatformContext): EditorEngine {
    val path = NSBundle.mainBundle.pathForResource("icu", "zst")!!
    val icu = NSData.create(contentsOfFile = path)!!
    return IosEditorEngine(NativeEditorEngine().also { it.loadIcuData(icu, null) })
  }
}

private class IosEditorEngine(private val native: NativeEditorEngine) : EditorEngine {
  override fun createEditor(scaleFactor: Double, snapshot: ByteArray?): Editor {
    val editor = native.createEditorWithScaleFactor(
      scaleFactor, snapshot = snapshot?.toNSData(), error = null
    )!!
    return IosEditor(editor)
  }

  override fun close() {}
}

private class IosEditor(private val native: NativeEditor) : Editor {
  override fun dispatch(messageJson: String) {
    native.dispatchWithMessageJson(messageJson, null)
  }

  override fun tick() {
    native.tickAndReturnError(null)
  }

  override fun exportSnapshot(): ByteArray {
    return native.exportSnapshotAndReturnError(null)!!.toByteArray()
  }

  override fun close() {}
}

private fun ByteArray.toNSData(): NSData = memScoped {
  NSData.create(bytes = allocArrayOf(this@toNSData), length = size.toULong())
}

private fun NSData.toByteArray(): ByteArray {
  val byteArray = ByteArray(length.toInt())
  byteArray.usePinned { pinned ->
    memcpy(pinned.addressOf(0), bytes, length)
  }
  return byteArray
}
