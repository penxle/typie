package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.IntOffset
import co.typie.editor.Editor
import co.typie.editor.EditorViewportTransform
import co.typie.editor.ext.isCollapsed
import co.typie.editor.ffi.PageRect
import co.typie.editor.interaction.gestures.EditorSelectionHandleRadiusDp
import co.typie.editor.interaction.gestures.EditorSelectionHandleStemWidthDp
import co.typie.editor.interaction.gestures.EditorSelectionHandleTouchTargetDp
import co.typie.editor.interaction.gestures.EditorSelectionHandleType
import co.typie.editor.interaction.gestures.resolveSelectionHandleGeometry
import co.typie.editor.interaction.hasActiveTableCellSelection
import co.typie.editor.runtime.EditorUiState
import co.typie.ui.theme.AppTheme
import kotlin.math.roundToInt

@Composable
internal fun EditorSelectionHandleOverlay(
  editor: Editor,
  uiState: EditorUiState,
  editorRectInOverlay: Rect,
  density: Float,
) {
  if (!uiState.focused || editor.selection.isCollapsed() || hasActiveTableCellSelection(editor)) {
    return
  }

  val placements =
    resolveSelectionHandleOverlayPlacements(
      editor = editor,
      uiState = uiState,
      editorRectInOverlay = editorRectInOverlay,
      density = density,
    ) ?: return
  val color = AppTheme.colors.textDefault

  placements.forEach { placement -> EditorSelectionHandle(placement = placement, color = color) }
}

internal fun resolveSelectionHandleOverlayPlacements(
  editor: Editor,
  uiState: EditorUiState,
  editorRectInOverlay: Rect,
  density: Float,
): List<EditorSelectionHandleOverlayPlacement>? {
  if (density <= 0f || editor.selection.isCollapsed() || hasActiveTableCellSelection(editor)) {
    return null
  }

  val endpoints = editor.tickSelectionEndpoints ?: return null
  val transform = uiState.resolveViewportTransform(pageSizes = editor.pageSizes)
  val from =
    resolveSelectionHandleOverlayPlacement(
      type = EditorSelectionHandleType.From,
      endpoint = endpoints.from,
      transform = transform,
      editorRectInOverlay = editorRectInOverlay,
      density = density,
    ) ?: return null
  val to =
    resolveSelectionHandleOverlayPlacement(
      type = EditorSelectionHandleType.To,
      endpoint = endpoints.to,
      transform = transform,
      editorRectInOverlay = editorRectInOverlay,
      density = density,
    ) ?: return null
  return listOf(from, to)
}

internal data class EditorSelectionHandleOverlayPlacement(
  val type: EditorSelectionHandleType,
  val endpointTopLeftInOverlay: Offset,
  val stemHeightPx: Float,
)

internal fun resolveSelectionHandleOverlayGeometry(
  placement: EditorSelectionHandleOverlayPlacement,
  density: Float,
) =
  resolveSelectionHandleGeometry(
    type = placement.type,
    endpointTopLeftInOverlay = placement.endpointTopLeftInOverlay,
    stemHeightPx = placement.stemHeightPx,
    radiusPx = EditorSelectionHandleRadiusDp * density,
    stemWidthPx = EditorSelectionHandleStemWidthDp * density,
    touchTargetPx = EditorSelectionHandleTouchTargetDp * density,
  )

private fun resolveSelectionHandleOverlayPlacement(
  type: EditorSelectionHandleType,
  endpoint: PageRect,
  transform: EditorViewportTransform,
  editorRectInOverlay: Rect,
  density: Float,
): EditorSelectionHandleOverlayPlacement? {
  val rect = endpoint.rect
  val top = transform.localToGlobal(page = endpoint.pageIdx, x = rect.x, y = rect.y) ?: return null
  val bottom =
    transform.localToGlobal(page = endpoint.pageIdx, x = rect.x, y = rect.y + rect.height)
      ?: return null
  val topLeft =
    Offset(
      x = editorRectInOverlay.left + top.x * density,
      y = editorRectInOverlay.top + top.y * density,
    )
  val stemHeightPx = ((bottom.y - top.y) * density).coerceAtLeast(0f)
  return EditorSelectionHandleOverlayPlacement(
    type = type,
    endpointTopLeftInOverlay = topLeft,
    stemHeightPx = stemHeightPx,
  )
}

@Composable
private fun EditorSelectionHandle(placement: EditorSelectionHandleOverlayPlacement, color: Color) {
  val localDensity = LocalDensity.current
  val geometry = resolveSelectionHandleOverlayGeometry(placement, localDensity.density)

  Box(
    modifier =
      Modifier.offset {
          IntOffset(
            geometry.touchTargetTopLeft.x.roundToInt(),
            geometry.touchTargetTopLeft.y.roundToInt(),
          )
        }
        .size(
          width = with(localDensity) { geometry.touchTargetSize.width.toDp() },
          height = with(localDensity) { geometry.touchTargetSize.height.toDp() },
        )
  ) {
    Canvas(modifier = Modifier.matchParentSize()) {
      translate(
        left = geometry.paintTopLeftInTouchTarget.x,
        top = geometry.paintTopLeftInTouchTarget.y,
      ) {
        val centerX = geometry.radiusPx
        if (placement.type == EditorSelectionHandleType.From) {
          drawCircle(
            color = color,
            radius = geometry.radiusPx,
            center = Offset(centerX, geometry.radiusPx),
          )
          drawRect(
            color = color,
            topLeft = Offset(centerX - geometry.stemWidthPx / 2f, geometry.radiusPx * 2f),
            size = Size(geometry.stemWidthPx, geometry.stemHeightPx),
          )
        } else {
          drawRect(
            color = color,
            topLeft = Offset(centerX - geometry.stemWidthPx / 2f, 0f),
            size = Size(geometry.stemWidthPx, geometry.stemHeightPx),
          )
          drawCircle(
            color = color,
            radius = geometry.radiusPx,
            center = Offset(centerX, geometry.stemHeightPx + geometry.radiusPx),
          )
        }
      }
    }
  }
}
