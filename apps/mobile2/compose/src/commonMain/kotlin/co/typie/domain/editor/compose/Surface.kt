package co.typie.domain.editor.compose

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier

@Composable
internal expect fun Surface(
  modifier: Modifier,
  onAttach: (handle: Long) -> Unit,
  onDetach: () -> Unit,
  onResize: () -> Unit,
)
