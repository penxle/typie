package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier

@Composable
internal fun EditorScreenOverlayHost(modifier: Modifier = Modifier) {
  Box(modifier = modifier.fillMaxSize()) {
    // TODO(editor-parity): Populate screen/body overlays such as selection handles,
    // magnifier, scrollbar, and extension-area anchored affordances.
  }
}
