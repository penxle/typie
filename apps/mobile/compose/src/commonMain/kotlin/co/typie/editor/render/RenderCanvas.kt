package co.typie.editor.render

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.IntSize
import kotlinx.coroutines.flow.SharedFlow

@Composable
internal expect fun RenderCanvas(
  modifier: Modifier,
  desiredPixelSize: IntSize,
  trigger: SharedFlow<Long>,
  onAttach: (handle: Long) -> Unit,
  onDetach: (releaseBuffer: () -> Unit) -> Unit,
  onResize: () -> Unit,
  onBitmapCommitted: (pixelSize: IntSize, version: Long) -> Unit,
)

internal expect fun readNativeInts(srcAddr: Long, count: Int): IntArray
