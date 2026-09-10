package co.typie.editor.render

import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asComposeImageBitmap
import org.jetbrains.skia.Bitmap
import org.jetbrains.skia.ColorAlphaType
import org.jetbrains.skia.ColorType
import org.jetbrains.skia.ImageInfo
import org.jetbrains.skia.impl.use

internal actual fun copyRenderTile(
  handle: Long,
  index: Int,
  width: Int,
  height: Int,
): ImageBitmap? {
  val bitmap = Bitmap()
  if (!bitmap.allocPixels(ImageInfo(width, height, ColorType.RGBA_8888, ColorAlphaType.PREMUL))) {
    bitmap.close()
    return null
  }
  val address = bitmap.peekPixels()?.use(::skiaPixelAddress) ?: 0L
  if (
    address == 0L ||
      !RenderBuffer.readPinnedTileInto(handle, index, address, width.toLong() * height * 4)
  ) {
    bitmap.close()
    return null
  }
  bitmap.notifyPixelsChanged()
  return bitmap.asComposeImageBitmap()
}
