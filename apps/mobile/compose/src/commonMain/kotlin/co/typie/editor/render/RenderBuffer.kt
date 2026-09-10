package co.typie.editor.render

internal expect object RenderBuffer {
  fun allocate(): Long

  fun free(handle: Long): Unit

  fun beginRead(handle: Long): Boolean

  fun endRead(handle: Long): Unit

  fun getPixelWidth(handle: Long): Int

  fun getPixelHeight(handle: Long): Int

  fun getPinnedEditorRevision(handle: Long): Long

  fun getPinnedFrameKey(handle: Long): Long

  fun getPinnedTileCount(handle: Long): Int

  fun getPinnedTileBounds(handle: Long, index: Int): Long

  fun getPinnedTilePixels(handle: Long, index: Int): Long

  fun getPinnedTileVersion(handle: Long, index: Int): Long

  fun readPinnedTileInto(handle: Long, index: Int, dstAddr: Long, dstLen: Long): Boolean
}
