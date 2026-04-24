package co.typie.editor.render

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import kotlinx.coroutines.flow.SharedFlow

@Composable
internal expect fun RenderCanvas(
  modifier: Modifier,
  trigger: SharedFlow<Unit>,
  onAttach: (handle: Long) -> Unit,
  onDetach: () -> Unit,
  onResize: () -> Unit,
)

internal expect fun copyNativeBytes(srcAddr: Long, dst: ByteArray, length: Int)
