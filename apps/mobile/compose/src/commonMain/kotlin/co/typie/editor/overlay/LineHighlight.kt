package co.typie.editor.overlay

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
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

// Mixed read timing on purpose. The cursor is a composition-captured value so the
// highlight moves in the same recomposition wave as the page frames and overlays — a
// draw-phase read would observe a background-thread publish one frame ahead of them and
// flash one line off. The bounds and the viewport transform stay draw-phase reads: they
// come from post-layout position trackers, and capturing them in composition would lag
// layout-driven movement (zoom, relayout) by a frame.
internal fun Modifier.editorExtensionAreaLineHighlight(
  cursor: CursorMetrics?,
  focused: Boolean,
  editorBounds: () -> EditorBoundsInContainer,
  viewportTransform: () -> EditorViewportTransform,
  enabled: Boolean,
  color: Color,
): Modifier = drawBehind {
  if (!enabled || !focused) return@drawBehind
  val currentCursor = cursor ?: return@drawBehind
  val band =
    resolveEditorExtensionAreaLineHighlightBand(
      cursor = currentCursor,
      editorBounds = editorBounds(),
      viewportTransform = viewportTransform(),
    ) ?: return@drawBehind
  drawRect(
    color = color,
    topLeft = Offset(x = 0f, y = band.top * density),
    size = Size(width = size.width, height = band.height * density),
  )
}
