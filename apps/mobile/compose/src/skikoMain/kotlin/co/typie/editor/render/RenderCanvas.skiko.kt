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
  trigger: SharedFlow<Long>,
  onAttach: (handle: Long) -> Unit,
  onDetach: (releaseBuffer: () -> Unit) -> Unit,
  onResize: () -> Unit,
  onBitmapCommitted: (pixelSize: IntSize, version: Long) -> Unit,
) {
  var bufferHandle by remember { mutableStateOf(0L) }
  var bitmap by remember { mutableStateOf<ImageBitmap?>(null) }

  val currentOnAttach by rememberUpdatedState(onAttach)
  val currentOnDetach by rememberUpdatedState(onDetach)
  val currentOnResize by rememberUpdatedState(onResize)
  val currentOnBitmapCommitted by rememberUpdatedState(onBitmapCommitted)

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
    bitmap?.let {
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
    var cachedSkBitmap: Bitmap? = null
    var cachedPixelsAddr = 0L
    var readerLastVersion = 0L

    trigger.conflate().collect { version ->
      if (!RenderBuffer.beginRead(handle)) return@collect

      val w = RenderBuffer.getPixelWidth(handle)
      val h = RenderBuffer.getPixelHeight(handle)
      if (w <= 0 || h <= 0) {
        RenderBuffer.endRead(handle)
        return@collect
      }

      val hadBitmap = cachedSkBitmap != null
      val prevW = cachedWidth
      val prevH = cachedHeight
      val skBitmap =
        cachedSkBitmap?.takeIf { prevW == w && prevH == h }
          ?: run {
            val fresh = Bitmap()
            if (!fresh.allocPixels(ImageInfo(w, h, ColorType.RGBA_8888, ColorAlphaType.PREMUL))) {
              fresh.close()
              RenderBuffer.endRead(handle)
              return@collect
            }
            val addr = fresh.peekPixels()?.use { it.addr.toLong() } ?: 0L
            if (addr == 0L) {
              fresh.close()
              RenderBuffer.endRead(handle)
              return@collect
            }
            cachedSkBitmap = fresh
            cachedPixelsAddr = addr
            cachedWidth = w
            cachedHeight = h
            readerLastVersion = 0L
            fresh
          }

      val pinnedVersion = RenderBuffer.getPinnedVersion(handle)
      val damageFrom = RenderBuffer.getPinnedDamageFrom(handle)
      val damageCount = RenderBuffer.getPinnedDamageCount(handle)
      val damagePtr = RenderBuffer.getPinnedDamagePointer(handle)
      val partial =
        shouldPartialUpload(
          hadBitmap,
          prevW,
          prevH,
          w,
          h,
          readerLastVersion,
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
        RenderBuffer.readPinnedInto(handle, cachedPixelsAddr, w.toLong() * h * 4, rowFrom, rowTo)
      RenderBuffer.endRead(handle)
      if (!ok) return@collect

      skBitmap.notifyPixelsChanged()
      // asComposeImageBitmap() is zero-copy, so Compose may still draw this Bitmap after
      // it leaves the cache. Let Skiko's managed cleanup reclaim published bitmaps.
      bitmap = skBitmap.asComposeImageBitmap()
      currentOnBitmapCommitted(IntSize(w, h), version)
      readerLastVersion = pinnedVersion
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
