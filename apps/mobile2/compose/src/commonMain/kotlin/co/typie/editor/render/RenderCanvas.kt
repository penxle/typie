package co.typie.editor.render

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier

@Composable
internal expect fun RenderCanvas(
  modifier: Modifier,
  onAttach: (handle: Long) -> Unit,
  onDetach: () -> Unit,
  onResize: () -> Unit,
)

internal expect fun copyNativeBytes(srcAddr: Long, dst: ByteArray, length: Int)
