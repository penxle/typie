package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import co.typie.editor.Editor
import co.typie.editor.interaction.EditorInteractionGeometry
import co.typie.editor.interaction.semantics.EditorTableColumnResizePresentation
import co.typie.editor.interaction.semantics.resolveTableColumnResizePlacement
import co.typie.editor.interaction.semantics.resolveTableColumnResizePreviewDelta
import co.typie.editor.runtime.EditorUiState
import co.typie.ui.theme.AppTheme

private const val TableColumnResizeVisualWidthDp = 3f

@Composable
internal fun EditorTableColumnResizeOverlay(
  editor: Editor,
  uiState: EditorUiState,
  geometry: EditorInteractionGeometry,
  editorOffsetInSurface: Offset,
  presentation: EditorTableColumnResizePresentation,
) {
  val density = geometry.density
  if (!uiState.focused || density <= 0f) {
    return
  }

  val placement = resolveTableColumnResizePlacement(editor = editor, geometry = geometry) ?: return
  val color = AppTheme.colors.palette.blue
  val activeDraft = presentation.draft
  val resizeHandleActive = activeDraft != null || presentation.pressed
  val visualCenterX =
    editorOffsetInSurface.x +
      (activeDraft?.let {
        it.baseCenterX + resolveTableColumnResizePreviewDelta(it) * it.pxPerPageUnit
      } ?: placement.centerX)
  val visualTop = editorOffsetInSurface.y + (activeDraft?.top ?: placement.top)
  val visualBottom = editorOffsetInSurface.y + (activeDraft?.bottom ?: placement.bottom)
  val visualWidth = TableColumnResizeVisualWidthDp * density
  val verticalInset = 2f * density

  Box(modifier = Modifier.fillMaxSize()) {
    Canvas(modifier = Modifier.fillMaxSize()) {
      val height = (visualBottom - visualTop - verticalInset * 2f).coerceAtLeast(0f)
      if (height > 0f) {
        drawRoundRect(
          color = color.copy(alpha = if (resizeHandleActive) 0.85f else 0.35f),
          topLeft = Offset(x = visualCenterX - visualWidth / 2f, y = visualTop + verticalInset),
          size = Size(width = visualWidth, height = height),
          cornerRadius = CornerRadius(visualWidth / 2f, visualWidth / 2f),
        )
      }
    }
  }
}
