package co.typie.editor.render

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.IntSize
import kotlinx.coroutines.flow.SharedFlow

@Composable
internal expect fun RenderCanvas(
  modifier: Modifier,
  desiredPixelSize: IntSize,
  trigger: SharedFlow<Unit>,
  onAttach: (handle: Long) -> Unit,
  onDetach: () -> Unit,
  onResize: () -> Unit,
  onBitmapCommitted: (pixelSize: IntSize) -> Unit,
)

internal expect fun copyNativeBytes(srcAddr: Long, dst: ByteArray, length: Int)
