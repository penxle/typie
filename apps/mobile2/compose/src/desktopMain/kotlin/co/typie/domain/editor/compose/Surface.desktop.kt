package co.typie.domain.editor.compose

import androidx.compose.foundation.Canvas
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.toComposeImageBitmap
import androidx.compose.ui.layout.onSizeChanged
import com.sun.jna.Pointer
import org.jetbrains.skia.Bitmap
import org.jetbrains.skia.ColorAlphaType
import org.jetbrains.skia.ColorType
import org.jetbrains.skia.Image as SkImage
import org.jetbrains.skia.ImageInfo

@Composable
internal actual fun Surface(
  modifier: Modifier,
  onAttach: (handle: Long) -> Unit,
  onDetach: () -> Unit,
  onResize: () -> Unit,
) {
  var bufferHandle by remember { mutableStateOf(0L) }
  var bitmap by remember { mutableStateOf<ImageBitmap?>(null) }

  Canvas(
    modifier =
      modifier.onSizeChanged { size ->
        if (bufferHandle == 0L && size.width > 0 && size.height > 0) {
          bufferHandle = DesktopSurfaceBridge.allocatePixelBuffer(size.width, size.height)
          if (bufferHandle != 0L) {
            onAttach(bufferHandle)
            onResize()
          }
        } else if (bufferHandle != 0L) {
          onResize()
        }
      }
  ) {
    bitmap?.let { drawImage(it) }
  }

  LaunchedEffect(bufferHandle) {
    val handle = bufferHandle
    if (handle == 0L) return@LaunchedEffect

    while (true) {
      withFrameNanos {}
      if (!DesktopSurfaceBridge.checkAndClearDirty(handle)) continue

      val w = DesktopSurfaceBridge.getPixelWidth(handle)
      val h = DesktopSurfaceBridge.getPixelHeight(handle)
      if (w <= 0 || h <= 0) continue

      val dataAddr = DesktopSurfaceBridge.getDataPointer(handle)
      if (dataAddr == 0L) continue

      val ptr = Pointer(dataAddr)
      val bytes = ByteArray(w * h * 4)
      ptr.read(0, bytes, 0, bytes.size)

      val skBitmap = Bitmap()
      skBitmap.allocPixels(ImageInfo(w, h, ColorType.RGBA_8888, ColorAlphaType.PREMUL))
      skBitmap.installPixels(skBitmap.imageInfo, bytes, w * 4)
      val skImage = SkImage.makeFromBitmap(skBitmap)
      bitmap = skImage.toComposeImageBitmap()
    }
  }

  DisposableEffect(Unit) {
    onDispose {
      val handle = bufferHandle
      if (handle != 0L) {
        onDetach()
        DesktopSurfaceBridge.freePixelBuffer(handle)
        bufferHandle = 0L
      }
    }
  }
}
