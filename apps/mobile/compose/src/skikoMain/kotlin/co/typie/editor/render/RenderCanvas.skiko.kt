package co.typie.editor.render

import androidx.compose.foundation.Canvas
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asComposeImageBitmap
import androidx.compose.ui.unit.IntOffset
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
  var bufferHandle by remember { mutableStateOf(0L) }

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
    // Keep the displayed and delivered bitmaps immutable while the next frame copies.
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
          val w = RenderBuffer.getPixelWidth(handle)
          val h = RenderBuffer.getPixelHeight(handle)
          if (w <= 0 || h <= 0) {
            currentOnTargetUnavailable(expected)
            return@collect
          }

          if (cachedWidth != w || cachedHeight != h) {
            cachedSkBitmaps.fill(null)
            cachedImageBitmaps.fill(null)
            cachedPixelsAddrs.fill(0L)
            readerLastVersions.fill(0L)
            cachedWidth = w
            cachedHeight = h
          }

          // The published frame can remain visible while another page blocks publication.
          // Never mutate it, or the latest frame handed to the parent, in place.
          val retained = currentRetainedFrames()
          val backingIndex =
            cachedSkBitmaps.indices.firstOrNull { index ->
              cachedSkBitmaps[index] == null ||
                (cachedImageBitmaps[index] !== currentFrame &&
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
              ?: run {
                val fresh = Bitmap()
                if (
                  !fresh.allocPixels(ImageInfo(w, h, ColorType.RGBA_8888, ColorAlphaType.PREMUL))
                ) {
                  fresh.close()
                  currentOnTargetUnavailable(expected)
                  return@collect
                }
                val addr = fresh.peekPixels()?.use { it.addr.toLong() } ?: 0L
                if (addr == 0L) {
                  fresh.close()
                  currentOnTargetUnavailable(expected)
                  return@collect
                }
                cachedSkBitmaps[backingIndex] = fresh
                cachedPixelsAddrs[backingIndex] = addr
                fresh
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
          var rowTo = h
          if (partial && damagePtr != 0L && damageCount.toLong() * 4 <= Int.MAX_VALUE) {
            val ints = readNativeInts(damagePtr, damageCount * 4)
            val rr = damageRowRange(ints, damageCount, h)
            if (rr.minY < rr.maxY) {
              rowFrom = rr.minY
              rowTo = rr.maxY
            }
          }
          val ok =
            RenderBuffer.readPinnedInto(
              handle,
              cachedPixelsAddrs[backingIndex],
              w.toLong() * h * 4,
              rowFrom,
              rowTo,
            )
          if (!ok) {
            currentOnTargetUnavailable(expected)
            return@collect
          }

          deliveredSkBitmap = skBitmap
          deliveredBackingIndex = backingIndex
          deliveredSize = IntSize(w, h)
          readerLastVersions[backingIndex] = pinnedVersion
        } finally {
          RenderBuffer.endRead(handle)
        }

        val skBitmap = deliveredSkBitmap
        val backingIndex = deliveredBackingIndex
        // asComposeImageBitmap() is zero-copy, so the backing must not be reused
        // while Compose can still draw it.
        skBitmap.notifyPixelsChanged()
        val image =
          cachedImageBitmaps[backingIndex]
            ?: skBitmap.asComposeImageBitmap().also { cachedImageBitmaps[backingIndex] = it }
        lastDeliveredFrame = image
        currentOnFrame(image, deliveredSize, deliveredEditorRevision, deliveredFrameKey)
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
