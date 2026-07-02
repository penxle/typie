package co.typie.editor.render

internal expect object RenderBuffer {
  fun allocate(width: Int, height: Int): Long

  fun free(handle: Long)

  fun resize(handle: Long, width: Int, height: Int)

  fun beginRead(handle: Long): Boolean

  fun endRead(handle: Long)

  fun getDataPointer(handle: Long): Long

  fun getPixelWidth(handle: Long): Int

  fun getPixelHeight(handle: Long): Int

  fun getPinnedVersion(handle: Long): Long

  fun getPinnedDamageFrom(handle: Long): Long

  fun getPinnedDamagePointer(handle: Long): Long

  fun getPinnedDamageCount(handle: Long): Int
}
