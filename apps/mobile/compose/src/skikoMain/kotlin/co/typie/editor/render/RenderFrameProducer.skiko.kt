package co.typie.editor.render

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asComposeImageBitmap
import androidx.compose.ui.unit.IntSize
import co.typie.editor.SurfaceConfiguration
import co.typie.editor.ffi.FrameKey
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.conflate
import org.jetbrains.skia.Bitmap
import org.jetbrains.skia.ColorAlphaType
import org.jetbrains.skia.ColorType
import org.jetbrains.skia.ImageInfo
import org.jetbrains.skia.impl.use

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
  var bufferHandle by remember { mutableStateOf(0L) }
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
    val cachedSkBitmaps = arrayOfNulls<Bitmap>(4)
    val cachedImageBitmaps = arrayOfNulls<ImageBitmap>(cachedSkBitmaps.size)
    val cachedPixelsAddrs = LongArray(cachedSkBitmaps.size)
    val readerLastVersions = LongArray(cachedSkBitmaps.size)
    var lastDeliveredFrame: ImageBitmap? = null

    trigger.conflate().collect { expected ->
      try {
        if (!RenderBuffer.beginRead(handle)) {
          currentOnFrameUnavailable(expected)
          return@collect
        }

        var deliveredSkBitmap: Bitmap? = null
        var deliveredBackingIndex = -1
        var deliveredSize = IntSize.Zero
        var deliveredEditorRevision = 0L
        var deliveredFrameKey = 0L
        try {
          deliveredEditorRevision = RenderBuffer.getPinnedEditorRevision(handle)
          deliveredFrameKey = RenderBuffer.getPinnedFrameKey(handle)
          val width = RenderBuffer.getPixelWidth(handle)
          val height = RenderBuffer.getPixelHeight(handle)
          if (width <= 0 || height <= 0) {
            currentOnTargetUnavailable(expected)
            return@collect
          }

          if (cachedWidth != width || cachedHeight != height) {
            cachedSkBitmaps.fill(null)
            cachedImageBitmaps.fill(null)
            cachedPixelsAddrs.fill(0L)
            readerLastVersions.fill(0L)
            cachedWidth = width
            cachedHeight = height
          }

          val retained = currentRetainedFrames()
          val backingIndex =
            cachedSkBitmaps.indices.firstOrNull { index ->
              cachedSkBitmaps[index] == null ||
                (cachedImageBitmaps[index] !== currentDisplayedFrame &&
                  cachedImageBitmaps[index] !== lastDeliveredFrame &&
                  retained.none { it === cachedImageBitmaps[index] })
            }
              ?: run {
                currentOnFrameUnavailable(expected)
                return@collect
              }
          val hadBitmap = cachedSkBitmaps[backingIndex] != null
          val skBitmap =
            cachedSkBitmaps[backingIndex]
              ?: Bitmap().also { fresh ->
                if (
                  !fresh.allocPixels(
                    ImageInfo(width, height, ColorType.RGBA_8888, ColorAlphaType.PREMUL)
                  )
                ) {
                  fresh.close()
                  currentOnTargetUnavailable(expected)
                  return@collect
                }
                val address = fresh.peekPixels()?.use(::skiaPixelAddress) ?: 0L
                if (address == 0L) {
                  fresh.close()
                  currentOnTargetUnavailable(expected)
                  return@collect
                }
                cachedSkBitmaps[backingIndex] = fresh
                cachedPixelsAddrs[backingIndex] = address
              }

          val pinnedVersion = RenderBuffer.getPinnedVersion(handle)
          val damageFrom = RenderBuffer.getPinnedDamageFrom(handle)
          val damageCount = RenderBuffer.getPinnedDamageCount(handle)
          val damagePtr = RenderBuffer.getPinnedDamagePointer(handle)
          val partial =
            shouldPartialUpload(
              hadBitmap,
              readerLastVersions[backingIndex],
              damageFrom,
              damageCount,
            )
          var rowFrom = 0
          var rowTo = height
          if (partial && damagePtr != 0L && damageCount.toLong() * 4 <= Int.MAX_VALUE) {
            val rows =
              damageRowRange(readNativeInts(damagePtr, damageCount * 4), damageCount, height)
            if (rows.minY < rows.maxY) {
              rowFrom = rows.minY
              rowTo = rows.maxY
            }
          }
          if (
            !RenderBuffer.readPinnedInto(
              handle,
              cachedPixelsAddrs[backingIndex],
              width.toLong() * height * 4,
              rowFrom,
              rowTo,
            )
          ) {
            currentOnTargetUnavailable(expected)
            return@collect
          }
          deliveredSkBitmap = skBitmap
          deliveredBackingIndex = backingIndex
          deliveredSize = IntSize(width, height)
          readerLastVersions[backingIndex] = pinnedVersion
        } finally {
          RenderBuffer.endRead(handle)
        }

        val skBitmap = deliveredSkBitmap
        skBitmap.notifyPixelsChanged()
        val image =
          cachedImageBitmaps[deliveredBackingIndex]
            ?: skBitmap.asComposeImageBitmap().also {
              cachedImageBitmaps[deliveredBackingIndex] = it
            }
        lastDeliveredFrame = image
        currentOnFrame(image, deliveredSize, deliveredEditorRevision, deliveredFrameKey)
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
