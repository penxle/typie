package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.EditorViewportTransform
import co.typie.editor.ext.isCollapsed
import co.typie.editor.ffi.PageRect
import co.typie.editor.interaction.EditorInteractionController
import co.typie.editor.interaction.gestures.EditorSelectionHandleType
import co.typie.editor.runtime.EditorUiState
import co.typie.ui.theme.AppTheme
import kotlin.math.max
import kotlin.math.roundToInt

@Composable
internal fun EditorSelectionHandleOverlay(
  editor: Editor,
  uiState: EditorUiState,
  editorRectInOverlay: Rect,
  density: Float,
  interactionController: EditorInteractionController,
) {
  if (!uiState.focused || editor.selection.isCollapsed()) {
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

  placements.forEach { placement ->
    EditorSelectionHandle(
      placement = placement,
      editorRectInOverlay = editorRectInOverlay,
      color = color,
      interactionController = interactionController,
    )
  }
}

internal fun resolveSelectionHandleOverlayPlacements(
  editor: Editor,
  uiState: EditorUiState,
  editorRectInOverlay: Rect,
  density: Float,
): List<EditorSelectionHandleOverlayPlacement>? {
  if (density <= 0f || editor.selection.isCollapsed()) {
    return null
  }

  val endpoints = editor.selectionEndpoints() ?: return null
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
private fun EditorSelectionHandle(
  placement: EditorSelectionHandleOverlayPlacement,
  editorRectInOverlay: Rect,
  color: Color,
  interactionController: EditorInteractionController,
) {
  val density = LocalDensity.current
  val radiusPx = with(density) { SelectionHandleRadius.toPx() }
  val stemWidthPx = with(density) { SelectionHandleStemWidth.toPx() }
  val touchTargetPx = with(density) { SelectionHandleTouchTarget.toPx() }
  val geometry =
    resolveSelectionHandleGeometry(
      type = placement.type,
      endpointTopLeftInOverlay = placement.endpointTopLeftInOverlay,
      stemHeightPx = placement.stemHeightPx,
      radiusPx = radiusPx,
      stemWidthPx = stemWidthPx,
      touchTargetPx = touchTargetPx,
    )
  val latestTouchTargetTopLeft = rememberUpdatedState(geometry.touchTargetTopLeft)
  val latestEditorRectInOverlay = rememberUpdatedState(editorRectInOverlay)

  Box(
    modifier =
      Modifier.offset {
          IntOffset(
            geometry.touchTargetTopLeft.x.roundToInt(),
            geometry.touchTargetTopLeft.y.roundToInt(),
          )
        }
        .size(
          width = with(density) { geometry.touchTargetSize.width.toDp() },
          height = with(density) { geometry.touchTargetSize.height.toDp() },
        )
        .pointerInput(placement.type, interactionController) {
          with(interactionController.selectionHandleGesture) {
            detectDrag(
              type = placement.type,
              positionInEditor = { localPosition ->
                latestTouchTargetTopLeft.value + localPosition -
                  latestEditorRectInOverlay.value.topLeft
              },
            )
          }
        }
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

internal data class EditorSelectionHandleGeometry(
  val touchTargetTopLeft: Offset,
  val touchTargetSize: Size,
  val paintTopLeftInTouchTarget: Offset,
  val stemHeightPx: Float,
  val radiusPx: Float,
  val stemWidthPx: Float,
)

internal fun resolveSelectionHandleGeometry(
  type: EditorSelectionHandleType,
  endpointTopLeftInOverlay: Offset,
  stemHeightPx: Float,
  radiusPx: Float,
  stemWidthPx: Float,
  touchTargetPx: Float,
): EditorSelectionHandleGeometry {
  val totalHeightPx = radiusPx * 2f + stemHeightPx
  val effectiveTouchHeightPx = max(totalHeightPx, touchTargetPx)
  val customPaintTop = if (type == EditorSelectionHandleType.From) -radiusPx * 2f else 0f
  val handleCenterY = customPaintTop + totalHeightPx / 2f
  val touchTargetTop = handleCenterY - effectiveTouchHeightPx / 2f
  val handleXOffset =
    if (type == EditorSelectionHandleType.From) {
      -stemWidthPx / 2f
    } else {
      stemWidthPx / 2f
    }
  val touchTargetLeft = handleXOffset - touchTargetPx / 2f

  return EditorSelectionHandleGeometry(
    touchTargetTopLeft = endpointTopLeftInOverlay + Offset(touchTargetLeft, touchTargetTop),
    touchTargetSize = Size(width = touchTargetPx, height = effectiveTouchHeightPx),
    paintTopLeftInTouchTarget =
      Offset(x = (touchTargetPx - radiusPx * 2f) / 2f, y = customPaintTop - touchTargetTop),
    stemHeightPx = stemHeightPx,
    radiusPx = radiusPx,
    stemWidthPx = stemWidthPx,
  )
}

private val SelectionHandleRadius = 8.dp
private val SelectionHandleStemWidth = 2.dp
private val SelectionHandleTouchTarget = 44.dp
