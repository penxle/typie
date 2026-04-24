package co.typie.editor.render

import androidx.compose.foundation.Canvas
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.toComposeImageBitmap
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import kotlinx.coroutines.flow.SharedFlow
import org.jetbrains.skia.Bitmap
import org.jetbrains.skia.ColorAlphaType
import org.jetbrains.skia.ColorType
import org.jetbrains.skia.Image as SkImage
import org.jetbrains.skia.ImageInfo

@Composable
internal actual fun RenderCanvas(
  modifier: Modifier,
  trigger: SharedFlow<Unit>,
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
          bufferHandle = RenderBuffer.allocate(size.width, size.height)
          if (bufferHandle != 0L) {
            onAttach(bufferHandle)
            onResize()
          }
        } else if (bufferHandle != 0L) {
          onResize()
        }
      }
  ) {
    bitmap?.let {
      drawImage(
        image = it,
        srcOffset = IntOffset.Zero,
        srcSize = IntSize(it.width, it.height),
        dstOffset = IntOffset.Zero,
        dstSize = IntSize(size.width.toInt(), size.height.toInt()),
      )
    }
  }

  LaunchedEffect(bufferHandle) {
    val handle = bufferHandle
    if (handle == 0L) return@LaunchedEffect

    var cachedWidth = 0
    var cachedHeight = 0
    var cachedBytes: ByteArray? = null
    var cachedSkBitmap: Bitmap? = null

    trigger.collect {
      if (!RenderBuffer.beginRead(handle)) return@collect

      val w = RenderBuffer.getPixelWidth(handle)
      val h = RenderBuffer.getPixelHeight(handle)
      val dataAddr = RenderBuffer.getDataPointer(handle)
      if (w <= 0 || h <= 0 || dataAddr == 0L) {
        RenderBuffer.endRead(handle)
        return@collect
      }

      val byteCount = w * h * 4
      val bytes =
        cachedBytes?.takeIf { it.size == byteCount }
          ?: ByteArray(byteCount).also { cachedBytes = it }
      copyNativeBytes(dataAddr, bytes, byteCount)
      RenderBuffer.endRead(handle)

      val skBitmap =
        cachedSkBitmap?.takeIf { cachedWidth == w && cachedHeight == h }
          ?: Bitmap()
            .apply { allocPixels(ImageInfo(w, h, ColorType.RGBA_8888, ColorAlphaType.PREMUL)) }
            .also {
              cachedSkBitmap = it
              cachedWidth = w
              cachedHeight = h
            }
      skBitmap.installPixels(skBitmap.imageInfo, bytes, w * 4)
      bitmap = SkImage.makeFromBitmap(skBitmap).toComposeImageBitmap()
    }
  }

  DisposableEffect(Unit) {
    onDispose {
      val handle = bufferHandle
      if (handle != 0L) {
        onDetach()
        RenderBuffer.free(handle)
        bufferHandle = 0L
      }
    }
  }
}
