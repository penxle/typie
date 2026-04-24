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
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import androidx.core.graphics.createBitmap
import com.sun.jna.Pointer
import java.nio.ByteBuffer
import kotlinx.coroutines.flow.SharedFlow

@Composable
internal actual fun RenderCanvas(
  modifier: Modifier,
  desiredPixelSize: IntSize,
  trigger: SharedFlow<Unit>,
  onAttach: (handle: Long) -> Unit,
  onDetach: () -> Unit,
  onResize: () -> Unit,
  onBitmapCommitted: (pixelSize: IntSize) -> Unit,
) {
  var bufferHandle by remember { mutableLongStateOf(0L) }
  var imageBitmap by remember { mutableStateOf<ImageBitmap?>(null) }

  val currentOnAttach by rememberUpdatedState(onAttach)
  val currentOnResize by rememberUpdatedState(onResize)
  val currentOnBitmapCommitted by rememberUpdatedState(onBitmapCommitted)

  LaunchedEffect(desiredPixelSize) {
    if (desiredPixelSize.width <= 0 || desiredPixelSize.height <= 0) return@LaunchedEffect
    if (bufferHandle == 0L) {
      val handle = RenderBuffer.allocate(desiredPixelSize.width, desiredPixelSize.height)
      if (handle == 0L) return@LaunchedEffect
      bufferHandle = handle
      currentOnAttach(handle)
      currentOnResize()
    } else {
      currentOnResize()
    }
  }

  Canvas(modifier = modifier) {
    imageBitmap?.let {
      drawImage(
        image = it,
        srcOffset = IntOffset.Zero,
        srcSize = IntSize(it.width, it.height),
        dstOffset = IntOffset.Zero,
        dstSize = IntSize(it.width, it.height),
      )
    }
  }

  LaunchedEffect(bufferHandle) {
    val handle = bufferHandle
    if (handle == 0L) return@LaunchedEffect

    var cachedWidth = 0
    var cachedHeight = 0
    var cachedBytes: ByteArray? = null
    var cachedAndroidBitmap: Bitmap? = null

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
      currentOnBitmapCommitted(IntSize(w, h))
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
