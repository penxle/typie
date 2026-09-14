package co.typie.editor.overlay

import androidx.compose.animation.core.Animatable
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalCursorBlinkEnabled
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Rect
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.delay

private const val EditorCaretWidthDp = 2f

@Composable
internal fun EditorCursorOverlay(
  cursor: CursorMetrics?,
  focused: Boolean,
  displayZoom: Float,
  revision: Long,
) {
  if (!focused) {
    return
  }

  val currentCursor = cursor ?: return
  val rect = resolveEditorCursorOverlayRect(cursor = currentCursor, displayZoom = displayZoom)
  val alpha = remember { Animatable(1f) }
  val blinkEnabled = LocalCursorBlinkEnabled.current

  LaunchedEffect(revision, currentCursor, blinkEnabled) {
    alpha.snapTo(1f)
    if (!blinkEnabled) return@LaunchedEffect
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
      .background(AppTheme.colors.textDefault)
  )
}

internal fun resolveEditorCursorOverlayRect(cursor: CursorMetrics, displayZoom: Float): Rect =
  Rect(
    x = cursor.caret.x * displayZoom,
    y = cursor.caret.y * displayZoom,
    width = EditorCaretWidthDp,
    height = cursor.caret.height * displayZoom,
  )
