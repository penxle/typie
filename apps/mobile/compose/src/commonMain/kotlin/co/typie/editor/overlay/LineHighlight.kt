package co.typie.editor.overlay

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Rect
import co.typie.ui.theme.AppTheme

internal fun resolveEditorLineHighlightOverlayRect(
  cursor: CursorMetrics,
  pageWidth: Float,
  displayZoom: Float,
): Rect {
  val line = cursor.line
  return Rect(x = 0f, y = line.y, width = pageWidth, height = line.height).scale(displayZoom)
}

@Composable
internal fun EditorLineHighlightOverlay(
  cursor: CursorMetrics?,
  focused: Boolean,
  displayZoom: Float,
  pageWidth: Float,
  enabled: Boolean,
) {
  if (!enabled || !focused) {
    return
  }

  val currentCursor = cursor ?: return
  val rect =
    resolveEditorLineHighlightOverlayRect(
      cursor = currentCursor,
      pageWidth = pageWidth,
      displayZoom = displayZoom,
    )

  Box(Modifier.editorOverlayRect(rect).background(AppTheme.colors.surfaceInset.copy(alpha = 0.55f)))
}
