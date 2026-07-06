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
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.conflate

@Composable
internal actual fun RenderCanvas(
  modifier: Modifier,
  desiredPixelSize: IntSize,
  trigger: SharedFlow<Long>,
  onAttach: (handle: Long) -> Unit,
  onDetach: () -> Unit,
  onResize: () -> Unit,
  onBitmapCommitted: (pixelSize: IntSize, version: Long) -> Unit,
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
    var cachedAndroidBitmap: Bitmap? = null

    trigger.conflate().collect { version ->
      if (!RenderBuffer.beginRead(handle)) return@collect

      val w = RenderBuffer.getPixelWidth(handle)
      val h = RenderBuffer.getPixelHeight(handle)
      val dataAddr = RenderBuffer.getDataPointer(handle)
      if (w <= 0 || h <= 0 || dataAddr == 0L) {
        RenderBuffer.endRead(handle)
        return@collect
      }

      // ARGB_8888 stores bytes in R,G,B,A order in memory (despite the name) and is
      // premultiplied by default. This matches CpuSink::read_back_rect_absolute's
      // premultiplied RGBA8 output, so copyPixelsFromBuffer is a direct memcpy with
      // no channel swap or un-premultiplication.
      val androidBitmap =
        cachedAndroidBitmap?.takeIf { cachedWidth == w && cachedHeight == h }
          ?: createBitmap(w, h).also {
            cachedAndroidBitmap = it
            cachedWidth = w
            cachedHeight = h
          }
      val byteCount = androidBitmap.byteCount.toLong()
      if (byteCount != w.toLong() * h * 4) {
        RenderBuffer.endRead(handle)
        return@collect
      }
      androidBitmap.copyPixelsFromBuffer(Pointer(dataAddr).getByteBuffer(0, byteCount))
      RenderBuffer.endRead(handle)

      imageBitmap = androidBitmap.asImageBitmap()
      currentOnBitmapCommitted(IntSize(w, h), version)
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

internal actual fun readNativeInts(srcAddr: Long, count: Int): IntArray =
  Pointer(srcAddr).getIntArray(0, count)
