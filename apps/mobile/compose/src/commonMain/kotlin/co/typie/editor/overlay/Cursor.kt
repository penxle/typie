package co.typie.editor.overlay

import androidx.compose.animation.core.Animatable
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Rect
import kotlinx.coroutines.delay

@Composable
internal fun EditorCursorOverlay(cursor: CursorMetrics?, focused: Boolean, displayZoom: Float) {
  if (!focused) {
    return
  }

  val currentCursor = cursor ?: return
  val rect = resolveEditorCursorOverlayRect(cursor = currentCursor, displayZoom = displayZoom)
  val alpha = remember { Animatable(1f) }

  LaunchedEffect(rect.x, rect.y) {
    alpha.snapTo(1f)
    while (true) {
      delay(500)
      alpha.snapTo(0f)
      delay(500)
      alpha.snapTo(1f)
    }
  }

  Box(
    Modifier.editorOverlayRect(rect)
      .graphicsLayer { this.alpha = alpha.value }
      .background(Color.Black)
  )
}

internal fun resolveEditorCursorOverlayRect(cursor: CursorMetrics, displayZoom: Float): Rect =
  cursor.caret.scale(displayZoom)
