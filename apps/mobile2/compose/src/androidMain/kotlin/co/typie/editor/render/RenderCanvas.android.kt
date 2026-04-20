package co.typie.editor.render

import android.graphics.Bitmap
import androidx.compose.foundation.Canvas
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.onSizeChanged
import androidx.core.graphics.createBitmap
import com.sun.jna.Pointer
import java.nio.ByteBuffer

@Composable
internal actual fun RenderCanvas(
  modifier: Modifier,
  onAttach: (handle: Long) -> Unit,
  onDetach: () -> Unit,
  onResize: () -> Unit,
) {
  var bufferHandle by remember { mutableLongStateOf(0L) }
  var imageBitmap by remember { mutableStateOf<ImageBitmap?>(null) }

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
    imageBitmap?.let { drawImage(image = it) }
  }

  LaunchedEffect(bufferHandle) {
    val handle = bufferHandle
    if (handle == 0L) return@LaunchedEffect

    var cachedWidth = 0
    var cachedHeight = 0
    var cachedBytes: ByteArray? = null
    var cachedAndroidBitmap: Bitmap? = null

    while (true) {
      withFrameNanos {}
      if (!RenderBuffer.beginRead(handle)) continue

      val w = RenderBuffer.getPixelWidth(handle)
      val h = RenderBuffer.getPixelHeight(handle)
      val dataAddr = RenderBuffer.getDataPointer(handle)
      if (w <= 0 || h <= 0 || dataAddr == 0L) {
        RenderBuffer.endRead(handle)
        continue
      }

      val byteCount = w * h * 4
      val bytes =
        cachedBytes?.takeIf { it.size == byteCount }
          ?: ByteArray(byteCount).also { cachedBytes = it }
      copyNativeBytes(dataAddr, bytes, byteCount)
      RenderBuffer.endRead(handle)

      // ARGB_8888 stores bytes in R,G,B,A order in memory (despite the name) and is
      // premultiplied by default. This matches CpuSink::flush_to's premultiplied RGBA8
      // output, so copyPixelsFromBuffer is a direct memcpy with no channel swap or
      // un-premultiplication.
      val androidBitmap =
        cachedAndroidBitmap?.takeIf { cachedWidth == w && cachedHeight == h }
          ?: createBitmap(w, h).also {
            cachedAndroidBitmap = it
            cachedWidth = w
            cachedHeight = h
          }
      androidBitmap.copyPixelsFromBuffer(ByteBuffer.wrap(bytes))
      imageBitmap = androidBitmap.asImageBitmap()
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

internal actual fun copyNativeBytes(srcAddr: Long, dst: ByteArray, length: Int) {
  Pointer(srcAddr).read(0, dst, 0, length)
}
