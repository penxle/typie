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
import co.typie.editor.ffi.FrameKey
import com.sun.jna.Pointer
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.conflate

private data class AndroidFrameBacking(val bitmap: Bitmap, val image: ImageBitmap)

@Composable
internal actual fun RenderCanvas(
  modifier: Modifier,
  desiredPixelSize: IntSize,
  configuration: SurfaceConfiguration,
  frame: ImageBitmap?,
  retainedFrames: () -> List<ImageBitmap>,
  trigger: SharedFlow<FrameKey>,
  onAttach: (handle: Long) -> Unit,
  onDetach: (releaseBuffer: () -> Unit) -> Unit,
  onResize: suspend () -> Unit,
  onFrame: (bitmap: ImageBitmap, pixelSize: IntSize, editorRevision: Long, frameKey: Long) -> Unit,
  onFrameUnavailable: (FrameKey) -> Unit,
  onTargetUnavailable: (FrameKey?) -> Unit,
  onFailure: (Throwable) -> Unit,
) {
  var bufferHandle by remember { mutableLongStateOf(0L) }

  val currentOnAttach by rememberUpdatedState(onAttach)
  val currentOnDetach by rememberUpdatedState(onDetach)
  val currentOnResize by rememberUpdatedState(onResize)
  val currentOnFrame by rememberUpdatedState(onFrame)
  val currentOnFrameUnavailable by rememberUpdatedState(onFrameUnavailable)
  val currentOnTargetUnavailable by rememberUpdatedState(onTargetUnavailable)
  val currentOnFailure by rememberUpdatedState(onFailure)
  val currentFrame by rememberUpdatedState(frame)
  val currentRetainedFrames by rememberUpdatedState(retainedFrames)

  LaunchedEffect(desiredPixelSize, configuration) {
    try {
      if (desiredPixelSize.width <= 0 || desiredPixelSize.height <= 0) return@LaunchedEffect
      if (bufferHandle == 0L) {
        val handle = RenderBuffer.allocate(desiredPixelSize.width, desiredPixelSize.height)
        if (handle == 0L) {
          currentOnTargetUnavailable(null)
          return@LaunchedEffect
        }
        bufferHandle = handle
        currentOnAttach(handle)
        currentOnResize()
      } else {
        currentOnResize()
      }
    } catch (e: CancellationException) {
      throw e
    } catch (e: Throwable) {
      currentOnFailure(e)
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
    val cachedBackings = arrayOfNulls<AndroidFrameBacking>(4)
    var lastDeliveredFrame: ImageBitmap? = null

    trigger.conflate().collect { expected ->
      try {
        if (!RenderBuffer.beginRead(handle)) {
          currentOnFrameUnavailable(expected)
          return@collect
        }

        var deliveredBacking: AndroidFrameBacking? = null
        var deliveredSize = IntSize.Zero
        var deliveredEditorRevision = 0L
        var deliveredFrameKey = 0L
        try {
          deliveredEditorRevision = RenderBuffer.getPinnedEditorRevision(handle)
          deliveredFrameKey = RenderBuffer.getPinnedFrameKey(handle)
          val w = RenderBuffer.getPixelWidth(handle)
          val h = RenderBuffer.getPixelHeight(handle)
          val dataAddr = RenderBuffer.getDataPointer(handle)
          if (w <= 0 || h <= 0 || dataAddr == 0L) {
            currentOnTargetUnavailable(expected)
            return@collect
          }

          // ARGB_8888 stores bytes in R,G,B,A order in memory (despite the name) and is
          // premultiplied by default. This matches CpuSink::read_back_rect_absolute's
          // premultiplied RGBA8 output, so copyPixelsFromBuffer is a direct memcpy with
          // no channel swap or un-premultiplication.
          if (cachedWidth != w || cachedHeight != h) {
            cachedBackings.fill(null)
            cachedWidth = w
            cachedHeight = h
          }
          // The published frame can remain visible while another page blocks publication.
          // Never mutate it, or the latest frame handed to the parent, in place.
          val retained = currentRetainedFrames()
          val backingIndex =
            cachedBackings.indices.firstOrNull { index ->
              val candidate = cachedBackings[index]
              candidate == null ||
                (candidate.image !== currentFrame &&
                  candidate.image !== lastDeliveredFrame &&
                  retained.none { it === candidate.image })
            }
              ?: run {
                currentOnFrameUnavailable(expected)
                return@collect
              }
          val backing =
            cachedBackings[backingIndex]
              ?: createBitmap(w, h).let { bitmap ->
                AndroidFrameBacking(bitmap = bitmap, image = bitmap.asImageBitmap()).also {
                  cachedBackings[backingIndex] = it
                }
              }
          val byteCount = backing.bitmap.byteCount.toLong()
          if (byteCount != w.toLong() * h * 4) {
            currentOnTargetUnavailable(expected)
            return@collect
          }

          backing.bitmap.copyPixelsFromBuffer(Pointer(dataAddr).getByteBuffer(0, byteCount))
          deliveredBacking = backing
          deliveredSize = IntSize(w, h)
        } finally {
          RenderBuffer.endRead(handle)
        }

        val backing = deliveredBacking
        lastDeliveredFrame = backing.image
        currentOnFrame(backing.image, deliveredSize, deliveredEditorRevision, deliveredFrameKey)
        if (expected.value != deliveredFrameKey) {
          currentOnFrameUnavailable(expected)
        }
      } catch (e: CancellationException) {
        throw e
      } catch (e: Throwable) {
        currentOnFailure(e)
      }
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
