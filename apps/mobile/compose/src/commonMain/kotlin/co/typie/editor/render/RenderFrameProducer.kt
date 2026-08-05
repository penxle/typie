package co.typie.editor.render

import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.unit.IntSize
import co.typie.editor.SurfaceConfiguration
import co.typie.editor.ffi.FrameKey
import kotlinx.coroutines.flow.SharedFlow

@Composable
internal expect fun RenderFrameProducer(
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
)

internal expect fun skiaPixelAddress(pixelMap: Any): Long

internal expect fun readNativeInts(srcAddr: Long, count: Int): IntArray
