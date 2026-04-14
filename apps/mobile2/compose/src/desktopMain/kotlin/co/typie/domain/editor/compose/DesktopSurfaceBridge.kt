package co.typie.domain.editor.compose

import java.io.File

internal object DesktopSurfaceBridge {
  init {
    val jnaPath = System.getProperty("jna.library.path") ?: error("jna.library.path not set")
    val libFile = File(jnaPath, "libeditor_ffi.dylib")
    System.load(libFile.absolutePath)
  }

  external fun allocatePixelBuffer(width: Int, height: Int): Long

  external fun freePixelBuffer(ptr: Long)

  external fun getDataPointer(ptr: Long): Long

  external fun getPixelWidth(ptr: Long): Int

  external fun getPixelHeight(ptr: Long): Int

  external fun checkAndClearDirty(ptr: Long): Boolean
}
