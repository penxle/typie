package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.drawscope.Stroke
import co.typie.editor.Editor
import co.typie.editor.EditorViewportTransform
import co.typie.editor.interaction.EditorTableCellSelection
import co.typie.editor.interaction.EditorTableCellSelectionBorderWidthDp
import co.typie.editor.interaction.EditorTableCellSelectionHandleRadiusDp
import co.typie.editor.interaction.resolveTableCellSelections
import co.typie.editor.runtime.EditorUiState
import co.typie.ui.theme.AppTheme

@Composable
internal fun EditorTableCellSelectionOverlay(
  editor: Editor,
  uiState: EditorUiState,
  editorRectInOverlay: Rect,
  density: Float,
) {
  if (!uiState.focused) {
    return
  }

  val placements =
    resolveTableCellSelectionOverlayPlacements(
      editor = editor,
      uiState = uiState,
      editorRectInOverlay = editorRectInOverlay,
      density = density,
    )
  if (placements.isEmpty()) {
    return
  }
  val color = AppTheme.colors.textDefault

  Canvas(modifier = Modifier.fillMaxSize()) {
    placements.forEach { placement ->
      drawRect(
        color = color,
        topLeft = placement.outline.topLeft,
        size = placement.outline.size,
        style = Stroke(width = placement.borderWidthPx),
      )
      placement.handleCenter?.let { handleCenter ->
        drawCircle(color = color, radius = placement.handleRadiusPx, center = handleCenter)
      }
    }
  }
}

internal fun resolveTableCellSelectionOverlayPlacements(
  editor: Editor,
  uiState: EditorUiState,
  editorRectInOverlay: Rect,
  density: Float,
): List<EditorTableCellSelectionOverlayPlacement> {
  if (density <= 0f) {
    return emptyList()
  }

  val transform = uiState.resolveViewportTransform(pageSizes = editor.pageSizes)
  return resolveTableCellSelections(editor).mapNotNull { activeSelection ->
    val outline =
      resolveOutlineInOverlay(
        activeSelection = activeSelection,
        transform = transform,
        editorRectInOverlay = editorRectInOverlay,
        density = density,
      ) ?: return@mapNotNull null
    val handleCenter =
      activeSelection.geometry.handleCenter?.let { center ->
        resolvePositionInOverlay(
          activeSelection = activeSelection,
          x = center.x,
          y = center.y,
          transform = transform,
          editorRectInOverlay = editorRectInOverlay,
          density = density,
        )
      }

    EditorTableCellSelectionOverlayPlacement(
      outline = outline,
      handleCenter = handleCenter,
      borderWidthPx = EditorTableCellSelectionBorderWidthDp * density,
      handleRadiusPx = EditorTableCellSelectionHandleRadiusDp * density,
    )
  }
}

internal data class EditorTableCellSelectionOverlayPlacement(
  val outline: Rect,
  val handleCenter: Offset?,
  val borderWidthPx: Float,
  val handleRadiusPx: Float,
)

private fun resolveOutlineInOverlay(
  activeSelection: EditorTableCellSelection,
  transform: EditorViewportTransform,
  editorRectInOverlay: Rect,
  density: Float,
): Rect? {
  val outline = activeSelection.geometry.outline
  val topLeft =
    resolvePositionInOverlay(
      activeSelection = activeSelection,
      x = outline.left,
      y = outline.top,
      transform = transform,
      editorRectInOverlay = editorRectInOverlay,
      density = density,
    ) ?: return null
  val bottomRight =
    resolvePositionInOverlay(
      activeSelection = activeSelection,
      x = outline.right,
      y = outline.bottom,
      transform = transform,
      editorRectInOverlay = editorRectInOverlay,
      density = density,
    ) ?: return null
  return Rect(topLeft = topLeft, bottomRight = bottomRight)
}

private fun resolvePositionInOverlay(
  activeSelection: EditorTableCellSelection,
  x: Float,
  y: Float,
  transform: EditorViewportTransform,
  editorRectInOverlay: Rect,
  density: Float,
): Offset? {
  val position =
    transform.localToGlobal(page = activeSelection.overlay.pageIdx, x = x, y = y) ?: return null
  return Offset(
    x = editorRectInOverlay.left + position.x * density,
    y = editorRectInOverlay.top + position.y * density,
  )
}
