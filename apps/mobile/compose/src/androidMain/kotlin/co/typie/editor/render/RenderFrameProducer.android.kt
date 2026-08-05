package co.typie.editor.render

import android.graphics.Bitmap
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
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
internal actual fun RenderFrameProducer(
  desiredPixelSize: IntSize,
  configuration: SurfaceConfiguration,
  displayedFrame: ImageBitmap?,
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
  val currentDisplayedFrame by rememberUpdatedState(displayedFrame)
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
      }
      currentOnResize()
    } catch (error: CancellationException) {
      throw error
    } catch (error: Throwable) {
      currentOnFailure(error)
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
          val width = RenderBuffer.getPixelWidth(handle)
          val height = RenderBuffer.getPixelHeight(handle)
          val dataAddress = RenderBuffer.getDataPointer(handle)
          if (width <= 0 || height <= 0 || dataAddress == 0L) {
            currentOnTargetUnavailable(expected)
            return@collect
          }
          if (cachedWidth != width || cachedHeight != height) {
            cachedBackings.fill(null)
            cachedWidth = width
            cachedHeight = height
          }
          val retained = currentRetainedFrames()
          val backingIndex =
            cachedBackings.indices.firstOrNull { index ->
              val candidate = cachedBackings[index]
              candidate == null ||
                (candidate.image !== currentDisplayedFrame &&
                  candidate.image !== lastDeliveredFrame &&
                  retained.none { it === candidate.image })
            }
              ?: run {
                currentOnFrameUnavailable(expected)
                return@collect
              }
          val backing =
            cachedBackings[backingIndex]
              ?: createBitmap(width, height).let { bitmap ->
                AndroidFrameBacking(bitmap, bitmap.asImageBitmap()).also {
                  cachedBackings[backingIndex] = it
                }
              }
          val byteCount = backing.bitmap.byteCount.toLong()
          if (byteCount != width.toLong() * height * 4) {
            currentOnTargetUnavailable(expected)
            return@collect
          }
          backing.bitmap.copyPixelsFromBuffer(Pointer(dataAddress).getByteBuffer(0, byteCount))
          deliveredBacking = backing
          deliveredSize = IntSize(width, height)
        } finally {
          RenderBuffer.endRead(handle)
        }

        val backing = deliveredBacking
        lastDeliveredFrame = backing.image
        currentOnFrame(backing.image, deliveredSize, deliveredEditorRevision, deliveredFrameKey)
        if (expected.value != deliveredFrameKey) currentOnFrameUnavailable(expected)
      } catch (error: CancellationException) {
        throw error
      } catch (error: Throwable) {
        currentOnFailure(error)
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

internal actual fun skiaPixelAddress(pixelMap: Any): Long =
  error("Skia pixel storage is unavailable on Android")

internal actual fun readNativeInts(srcAddr: Long, count: Int): IntArray =
  Pointer(srcAddr).getIntArray(0, count)
