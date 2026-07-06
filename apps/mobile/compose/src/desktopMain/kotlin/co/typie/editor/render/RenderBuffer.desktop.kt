package co.typie.editor.render

import java.io.File

internal actual object RenderBuffer {
  init {
    val jnaPath = System.getProperty("jna.library.path") ?: error("jna.library.path not set")
    val libFile = File(jnaPath, "libeditor_ffi.dylib")
    @Suppress("UnsafeDynamicallyLoadedCode") System.load(libFile.absolutePath)
  }

  @JvmStatic actual external fun allocate(width: Int, height: Int): Long

  @JvmStatic actual external fun free(handle: Long)

  @JvmStatic actual external fun resize(handle: Long, width: Int, height: Int)

  @JvmStatic actual external fun beginRead(handle: Long): Boolean

  @JvmStatic actual external fun endRead(handle: Long)

  @JvmStatic actual external fun getDataPointer(handle: Long): Long

  @JvmStatic actual external fun getPixelWidth(handle: Long): Int

  @JvmStatic actual external fun getPixelHeight(handle: Long): Int

  @JvmStatic actual external fun getPinnedVersion(handle: Long): Long

  @JvmStatic actual external fun getPinnedDamageFrom(handle: Long): Long

  @JvmStatic actual external fun getPinnedDamagePointer(handle: Long): Long

  @JvmStatic actual external fun getPinnedDamageCount(handle: Long): Int

  @JvmStatic
  actual external fun readPinnedInto(
    handle: Long,
    dstAddr: Long,
    dstLen: Long,
    rowFrom: Int,
    rowTo: Int,
  ): Boolean
}
