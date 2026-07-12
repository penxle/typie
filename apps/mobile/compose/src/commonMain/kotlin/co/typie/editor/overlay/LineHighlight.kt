package co.typie.editor.overlay

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.dp
import co.typie.editor.EditorViewportTransform
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Rect
import co.typie.editor.runtime.EditorBoundsInContainer
import co.typie.ui.theme.AppTheme

internal data class EditorLineHighlightBand(val top: Float, val height: Float)

internal fun resolveEditorExtensionAreaLineHighlightBand(
  cursor: CursorMetrics,
  editorBounds: EditorBoundsInContainer,
  viewportTransform: EditorViewportTransform,
): EditorLineHighlightBand? {
  if (!editorBounds.isValid) return null
  val line = cursor.line
  if (!line.y.isFinite() || !line.height.isFinite() || line.height <= 0f) return null
  val linePosition =
    viewportTransform.localToGlobal(page = cursor.pageIdx, x = 0f, y = line.y) ?: return null
  val displayZoom = viewportTransform.displayZoom.takeIf { it.isFinite() && it > 0f } ?: 1f
  return EditorLineHighlightBand(
    top = editorBounds.y + linePosition.y,
    height = line.height * displayZoom,
  )
}

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

@Composable
internal fun EditorExtensionAreaLineHighlightOverlay(
  cursor: CursorMetrics?,
  focused: Boolean,
  editorBounds: EditorBoundsInContainer,
  viewportTransform: EditorViewportTransform,
  enabled: Boolean,
) {
  if (!enabled || !focused) return
  val currentCursor = cursor ?: return
  val band =
    resolveEditorExtensionAreaLineHighlightBand(
      cursor = currentCursor,
      editorBounds = editorBounds,
      viewportTransform = viewportTransform,
    ) ?: return

  Box(
    Modifier.fillMaxWidth()
      .graphicsLayer { translationY = band.top.dp.toPx() }
      .height(band.height.dp)
      .background(AppTheme.colors.surfaceInset.copy(alpha = 0.55f))
  )
}
