package co.typie.editor.render

import java.io.File

internal actual object RenderBuffer {
  init {
    val path = System.getProperty("jna.library.path") ?: error("jna.library.path not set")
    @Suppress("UnsafeDynamicallyLoadedCode")
    System.load(File(path, "libeditor_ffi.dylib").absolutePath)
  }

  @JvmStatic actual external fun allocate(): Long

  @JvmStatic actual external fun free(handle: Long): Unit

  @JvmStatic actual external fun beginRead(handle: Long): Boolean

  @JvmStatic actual external fun endRead(handle: Long): Unit

  @JvmStatic actual external fun getPixelWidth(handle: Long): Int

  @JvmStatic actual external fun getPixelHeight(handle: Long): Int

  @JvmStatic actual external fun getPinnedEditorRevision(handle: Long): Long

  @JvmStatic actual external fun getPinnedFrameKey(handle: Long): Long

  @JvmStatic actual external fun getPinnedTileCount(handle: Long): Int

  @JvmStatic actual external fun getPinnedTileBounds(handle: Long, index: Int): Long

  @JvmStatic actual external fun getPinnedTilePixels(handle: Long, index: Int): Long

  @JvmStatic actual external fun getPinnedTileVersion(handle: Long, index: Int): Long

  @JvmStatic
  actual external fun readPinnedTileInto(
    handle: Long,
    index: Int,
    dstAddr: Long,
    dstLen: Long,
  ): Boolean
}
