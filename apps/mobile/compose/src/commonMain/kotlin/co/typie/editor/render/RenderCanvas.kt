package co.typie.editor.render

import androidx.compose.runtime.Composable
import androidx.compose.runtime.key
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.unit.IntSize
import co.typie.editor.SurfaceConfiguration
import co.typie.editor.ffi.FrameKey
import kotlinx.coroutines.flow.SharedFlow

@Composable
internal fun RenderCanvasLifecycle(
  owner: Any,
  page: Int,
  instance: Int = 0,
  content: @Composable () -> Unit,
) {
  key(owner, page, instance) { content() }
}

@Composable
internal expect fun RenderCanvas(
  modifier: Modifier,
  desiredPixelSize: IntSize,
  configuration: SurfaceConfiguration,
  frame: ImageBitmap?,
  retainedFrames: () -> List<ImageBitmap>,
  trigger: SharedFlow<FrameKey>,
  onAttach: (handle: Long) -> Unit,
  onDetach: (releaseBuffer: () -> Unit) -> Unit,
  onResize: suspend () -> Unit,
  // Called only after the pinned frame has been copied into an immutable platform backing.
  onFrame: (bitmap: ImageBitmap, pixelSize: IntSize, editorRevision: Long, frameKey: Long) -> Unit,
  // The current target can wait for an explicit recovery signal.
  onFrameUnavailable: (FrameKey) -> Unit,
  // The current target cannot deliver this frame and must be replaced. A null key means
  // that platform backing creation failed before the Host target could be attached.
  onTargetUnavailable: (FrameKey?) -> Unit,
  // Unexpected native/platform delivery failures terminate the owning Editor.
  onFailure: (Throwable) -> Unit,
)

internal expect fun readNativeInts(srcAddr: Long, count: Int): IntArray
