package co.typie.editor.render

import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.core.graphics.createBitmap
import com.sun.jna.Pointer

internal actual fun copyRenderTile(
  handle: Long,
  index: Int,
  width: Int,
  height: Int,
): ImageBitmap? {
  val address = RenderBuffer.getPinnedTilePixels(handle, index)
  if (address == 0L) return null
  val bitmap = createBitmap(width, height)
  bitmap.copyPixelsFromBuffer(Pointer(address).getByteBuffer(0, width.toLong() * height * 4))
  bitmap.prepareToDraw()
  return bitmap.asImageBitmap()
}

internal actual fun readNativeInts(srcAddr: Long, count: Int): IntArray =
  Pointer(srcAddr).getIntArray(0, count)

internal actual fun skiaPixelAddress(pixelMap: Any): Long =
  error("Skia pixel storage is unavailable on Android")
