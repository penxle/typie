package co.typie.editor.render

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntRect
import androidx.compose.ui.unit.IntSize
import co.typie.editor.EditorTileGutter
import co.typie.editor.EditorTileSize
import co.typie.editor.MaximumSurfaceTiles
import co.typie.editor.PresentedTile
import co.typie.editor.SurfaceConfiguration
import co.typie.editor.ffi.FrameKey
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.conflate

private data class TilePixels(val bounds: IntRect, val version: Long)

@Composable
internal fun RenderFrameProducer(
  desiredPixelSize: IntSize,
  configuration: SurfaceConfiguration,
  trigger: SharedFlow<FrameKey>,
  onAttach: (handle: Long) -> Unit,
  onDetach: (releaseBuffer: () -> Unit) -> Unit,
  onResize: suspend () -> Unit,
  onFrame:
    (tiles: List<PresentedTile>, pixelSize: IntSize, editorRevision: Long, frameKey: Long) -> Unit,
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

  LaunchedEffect(desiredPixelSize, configuration) {
    try {
      if (desiredPixelSize.width <= 0 || desiredPixelSize.height <= 0) return@LaunchedEffect
      if (bufferHandle == 0L) {
        val handle = RenderBuffer.allocate()
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
    var cached = emptyMap<TilePixels, ImageBitmap>()
    trigger.conflate().collect { expected ->
      try {
        if (!RenderBuffer.beginRead(handle)) {
          currentOnFrameUnavailable(expected)
          return@collect
        }
        try {
          val revision = RenderBuffer.getPinnedEditorRevision(handle)
          val frameKey = RenderBuffer.getPinnedFrameKey(handle)
          val size =
            IntSize(RenderBuffer.getPixelWidth(handle), RenderBuffer.getPixelHeight(handle))
          val count = RenderBuffer.getPinnedTileCount(handle)
          if (size.width <= 0 || size.height <= 0 || count !in 0..MaximumSurfaceTiles) {
            currentOnTargetUnavailable(expected)
            return@collect
          }
          val next = mutableMapOf<TilePixels, ImageBitmap>()
          val tiles = ArrayList<PresentedTile>(count)
          repeat(count) { index ->
            val pointer = RenderBuffer.getPinnedTileBounds(handle, index)
            if (pointer == 0L) {
              currentOnTargetUnavailable(expected)
              return@collect
            }
            val rect = readNativeInts(pointer, 4).let { IntRect(it[0], it[1], it[2], it[3]) }
            if (rect.width !in 1..EditorTileSize || rect.height !in 1..EditorTileSize) {
              currentOnTargetUnavailable(expected)
              return@collect
            }
            val key = TilePixels(rect, RenderBuffer.getPinnedTileVersion(handle, index))
            val bitmap =
              cached[key]
                ?: copyRenderTile(
                  handle,
                  index,
                  rect.width + EditorTileGutter * 2,
                  rect.height + EditorTileGutter * 2,
                )
            if (bitmap == null) {
              currentOnTargetUnavailable(expected)
              return@collect
            }
            next[key] = bitmap
            tiles += PresentedTile(bitmap, IntOffset(rect.left, rect.top))
          }
          cached = next
          currentOnFrame(tiles, size, revision, frameKey)
          if (expected.value != frameKey) currentOnFrameUnavailable(expected)
        } finally {
          RenderBuffer.endRead(handle)
        }
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

internal expect fun copyRenderTile(handle: Long, index: Int, width: Int, height: Int): ImageBitmap?

internal expect fun skiaPixelAddress(pixelMap: Any): Long

internal expect fun readNativeInts(srcAddr: Long, count: Int): IntArray
