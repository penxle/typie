package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.dp
import co.typie.screen.editor.editor.layout.EditorVisibleArea
import co.typie.screen.editor.editor.scroll.EditorScrollPolicy

@Composable
internal fun EditorScreenOverlayHost(
  visibleArea: EditorVisibleArea,
  scrollPolicy: EditorScrollPolicy,
  modifier: Modifier = Modifier,
) {
  Box(modifier = modifier.fillMaxSize()) {
    DebugViewportLine(y = visibleArea.visibleViewportTop, color = Color(0xFF00C853))
    DebugViewportLine(y = visibleArea.visibleViewportBottom, color = Color(0xFF00C853))
    DebugViewportLine(y = scrollPolicy.keepVisibleRange.top, color = Color(0xFFFFAB00))
    DebugViewportLine(y = scrollPolicy.keepVisibleRange.bottom, color = Color(0xFFFFAB00))

    // TODO(editor-parity): Populate screen/body overlays such as selection handles,
    // magnifier, scrollbar, and extension-area anchored affordances.
  }
}

@Composable
private fun DebugViewportLine(y: Float, color: Color) {
  Box(
    modifier =
      Modifier.fillMaxWidth()
        .height(2.dp)
        .graphicsLayer { translationY = y.dp.toPx() }
        .background(color.copy(alpha = 0.9f))
  )
}
