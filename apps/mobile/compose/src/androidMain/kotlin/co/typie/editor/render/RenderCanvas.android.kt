package co.typie.editor.render

import android.graphics.Bitmap
import androidx.compose.foundation.Canvas
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import androidx.core.graphics.createBitmap
import co.typie.editor.SurfaceConfiguration
import com.sun.jna.Pointer
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.conflate

@Composable
internal actual fun RenderCanvas(
  modifier: Modifier,
  desiredPixelSize: IntSize,
  configuration: SurfaceConfiguration,
  frame: ImageBitmap?,
  trigger: SharedFlow<Long>,
  onAttach: (handle: Long) -> Unit,
  onDetach: (releaseBuffer: () -> Unit) -> Unit,
  onResize: () -> Unit,
  onFrame: (bitmap: ImageBitmap, pixelSize: IntSize, version: Long) -> Unit,
  onFrameSkipped: (version: Long) -> Unit,
) {
  var bufferHandle by remember { mutableLongStateOf(0L) }

  val currentOnAttach by rememberUpdatedState(onAttach)
  val currentOnDetach by rememberUpdatedState(onDetach)
  val currentOnResize by rememberUpdatedState(onResize)
  val currentOnFrame by rememberUpdatedState(onFrame)
  val currentOnFrameSkipped by rememberUpdatedState(onFrameSkipped)

  LaunchedEffect(desiredPixelSize, configuration) {
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
    frame?.let {
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
    // Ping-pong across three backings: a delivered frame's pixels must stay immutable
    // for as long as the presentation gate can still show it (the pending store keeps
    // up to two frames per size, plus the currently applied one). Reusing a single
    // bitmap would overwrite on-screen pixels in place, moving content ahead of the
    // publish invisibly to the gate.
    val cachedBitmaps = arrayOfNulls<Bitmap>(3)
    var cachedIndex = 0

    trigger.conflate().collect { version ->
      if (!RenderBuffer.beginRead(handle)) {
        // An earlier read already consumed the commit behind this present — its pixels
        // were in the slot that read pinned — so no frame will arrive for this version.
        // It must still settle, or a publish parked on it would sit until the watchdog.
        currentOnFrameSkipped(version)
        return@collect
      }

      val w = RenderBuffer.getPixelWidth(handle)
      val h = RenderBuffer.getPixelHeight(handle)
      val dataAddr = RenderBuffer.getDataPointer(handle)
      if (w <= 0 || h <= 0 || dataAddr == 0L) {
        RenderBuffer.endRead(handle)
        currentOnFrameSkipped(version)
        return@collect
      }

      // ARGB_8888 stores bytes in R,G,B,A order in memory (despite the name) and is
      // premultiplied by default. This matches CpuSink::read_back_rect_absolute's
      // premultiplied RGBA8 output, so copyPixelsFromBuffer is a direct memcpy with
      // no channel swap or un-premultiplication.
      if (cachedWidth != w || cachedHeight != h) {
        cachedBitmaps.fill(null)
        cachedIndex = 0
        cachedWidth = w
        cachedHeight = h
      }
      val androidBitmap =
        cachedBitmaps[cachedIndex] ?: createBitmap(w, h).also { cachedBitmaps[cachedIndex] = it }
      cachedIndex = (cachedIndex + 1) % cachedBitmaps.size
      val byteCount = androidBitmap.byteCount.toLong()
      if (byteCount != w.toLong() * h * 4) {
        RenderBuffer.endRead(handle)
        currentOnFrameSkipped(version)
        return@collect
      }
      androidBitmap.copyPixelsFromBuffer(Pointer(dataAddr).getByteBuffer(0, byteCount))
      RenderBuffer.endRead(handle)

      currentOnFrame(androidBitmap.asImageBitmap(), IntSize(w, h), version)
    }
  }

  DisposableEffect(Unit) {
    onDispose {
      val handle = bufferHandle
      if (handle != 0L) {
        bufferHandle = 0L
        currentOnDetach { RenderBuffer.free(handle) }
      }
    }
  }
}

internal actual fun readNativeInts(srcAddr: Long, count: Int): IntArray =
  Pointer(srcAddr).getIntArray(0, count)
