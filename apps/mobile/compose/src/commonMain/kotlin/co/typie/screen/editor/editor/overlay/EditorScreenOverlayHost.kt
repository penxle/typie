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
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.scroll.EditorAutoScrollMode
import co.typie.editor.scroll.EditorAutoScrollPolicy
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.viewport.EditorViewportState

@Composable
internal fun EditorScreenOverlayHost(
  viewportState: EditorViewportState,
  visibleArea: EditorVisibleArea,
  autoScrollPolicy: EditorAutoScrollPolicy,
  layoutSpec: EditorDocumentLayoutSpec,
  pageSizes: List<PageSize>,
  displayZoom: Float,
  showDebugOverlay: Boolean = false,
  modifier: Modifier = Modifier,
) {
  Box(modifier = modifier.fillMaxSize()) {
    EditorScrollbars(
      viewportState = viewportState,
      visibleArea = visibleArea,
      layoutSpec = layoutSpec,
      pageSizes = pageSizes,
      displayZoom = displayZoom,
      modifier = Modifier.fillMaxSize(),
    )

    if (showDebugOverlay) {
      DebugViewportLine(y = visibleArea.visibleViewportTop, color = Color(0xFF00C853))
      DebugViewportLine(y = visibleArea.visibleViewportBottom, color = Color(0xFF00C853))

      when (autoScrollPolicy.mode) {
        EditorAutoScrollMode.KeepCursorVisible -> {
          DebugViewportLine(y = autoScrollPolicy.keepVisibleRange.top, color = Color(0xFFFFAB00))
          DebugViewportLine(y = autoScrollPolicy.keepVisibleRange.bottom, color = Color(0xFFFFAB00))
        }

        EditorAutoScrollMode.Typewriter -> {
          autoScrollPolicy.targetTop?.let { DebugViewportLine(y = it, color = Color(0xFFFFAB00)) }
          autoScrollPolicy.targetBottom
            ?.takeIf { autoScrollPolicy.targetLineHeight > 0f }
            ?.let { DebugViewportLine(y = it, color = Color(0xFFFFAB00)) }
        }
      }
    }

    // TODO(editor-parity): selection handle, magnifier, extension area 기준
    // affordance 같은 screen/body 오버레이를 채워 넣어야 한다.
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
