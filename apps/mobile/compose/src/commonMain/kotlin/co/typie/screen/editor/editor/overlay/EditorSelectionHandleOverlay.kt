package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.drawscope.translate
import co.typie.editor.EditorState
import co.typie.editor.EditorViewportTransform
import co.typie.editor.ext.isCollapsed
import co.typie.editor.ffi.PageRect
import co.typie.editor.interaction.gestures.EditorSelectionHandleRadiusDp
import co.typie.editor.interaction.gestures.EditorSelectionHandleStemWidthDp
import co.typie.editor.interaction.gestures.EditorSelectionHandleTouchTargetDp
import co.typie.editor.interaction.gestures.EditorSelectionHandleType
import co.typie.editor.interaction.gestures.resolveSelectionHandleGeometry
import co.typie.editor.interaction.resolveTableCellSelections
import co.typie.editor.runtime.EditorUiState
import co.typie.ui.theme.AppTheme

@Composable
internal fun EditorSelectionHandleOverlay(
  state: EditorState,
  uiState: EditorUiState,
  density: Float,
  pagePresented: (Int) -> Boolean,
) {
  if (state.selection.isCollapsed() || resolveTableCellSelections(state).isNotEmpty()) {
    return
  }

  if (state.selectionEndpoints == null) {
    return
  }

  val color = AppTheme.colors.textDefault

  Canvas(modifier = Modifier.fillMaxSize()) {
    val editorRect = uiState.editorBoundsInContainer.toPxRect(density) ?: return@Canvas
    val placements =
      resolveSelectionHandleOverlayPlacements(
        state = state,
        uiState = uiState,
        editorRectInOverlay = editorRect,
        density = density,
        pagePresented = pagePresented,
      ) ?: return@Canvas
    placements.forEach { placement ->
      val geometry = resolveSelectionHandleOverlayGeometry(placement, density)
      translate(
        left = geometry.touchTargetTopLeft.x + geometry.paintTopLeftInTouchTarget.x,
        top = geometry.touchTargetTopLeft.y + geometry.paintTopLeftInTouchTarget.y,
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

internal fun resolveSelectionHandleOverlayPlacements(
  state: EditorState,
  uiState: EditorUiState,
  editorRectInOverlay: Rect,
  density: Float,
  pagePresented: (Int) -> Boolean = { true },
): List<EditorSelectionHandleOverlayPlacement>? {
  if (
    density <= 0f || state.selection.isCollapsed() || resolveTableCellSelections(state).isNotEmpty()
  ) {
    return null
  }

  val endpoints = state.selectionEndpoints ?: return null
  val transform = uiState.resolveViewportTransform(pageSizes = state.pageSizes)
  return buildList {
      if (pagePresented(endpoints.from.pageIdx)) {
        resolveSelectionHandleOverlayPlacement(
            type = EditorSelectionHandleType.From,
            endpoint = endpoints.from,
            transform = transform,
            editorRectInOverlay = editorRectInOverlay,
            density = density,
          )
          ?.let(::add)
      }
      if (pagePresented(endpoints.to.pageIdx)) {
        resolveSelectionHandleOverlayPlacement(
            type = EditorSelectionHandleType.To,
            endpoint = endpoints.to,
            transform = transform,
            editorRectInOverlay = editorRectInOverlay,
            density = density,
          )
          ?.let(::add)
      }
    }
    .takeIf { it.isNotEmpty() }
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
