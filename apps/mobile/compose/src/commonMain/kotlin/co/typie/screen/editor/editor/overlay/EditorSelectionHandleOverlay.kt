package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.absoluteOffset
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.key
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.layout.layout
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.IntOffset
import co.typie.editor.EditorState
import co.typie.editor.EditorViewportTransform
import co.typie.editor.ext.isCollapsed
import co.typie.editor.ffi.PageRect
import co.typie.editor.interaction.gestures.EditorAndroidCursorHandleRadiusDp
import co.typie.editor.interaction.gestures.EditorAndroidSelectionHandleRadiusDp
import co.typie.editor.interaction.gestures.EditorSelectionHandleGeometry
import co.typie.editor.interaction.gestures.EditorSelectionHandleRadiusDp
import co.typie.editor.interaction.gestures.EditorSelectionHandleStemWidthDp
import co.typie.editor.interaction.gestures.EditorSelectionHandleTouchTargetDp
import co.typie.editor.interaction.gestures.EditorSelectionHandleType
import co.typie.editor.interaction.gestures.resolveSelectionHandleGeometry
import co.typie.editor.interaction.resolveTableCellSelections
import co.typie.editor.runtime.EditorUiState
import co.typie.platform.Platform
import co.typie.ui.theme.AppTheme
import kotlin.math.roundToInt

@Composable
internal fun EditorSelectionHandleOverlay(
  state: EditorState,
  uiState: EditorUiState,
  density: Float,
  pagePresented: (Int) -> Boolean,
  directTouchInteraction: Boolean,
  platform: Platform,
  selectionHandlesHidden: Boolean,
) {
  val color = AppTheme.colors.textDefault
  val types =
    if (state.selection.isCollapsed()) listOf(EditorSelectionHandleType.Cursor)
    else listOf(EditorSelectionHandleType.From, EditorSelectionHandleType.To)
  Box(modifier = Modifier.fillMaxSize()) {
    types.forEach { type ->
      val image = uiState.selectionHandleImages[type]
      // Page positions change during layout; read them again for placement and drawing.
      fun geometry(): EditorSelectionHandleGeometry? {
        val editorRect = uiState.editorBoundsInContainer.toPxRect(density) ?: return null
        val placement =
          resolveSelectionHandleOverlayPlacements(
              state = state,
              uiState = uiState,
              editorRectInOverlay = editorRect,
              density = density,
              pagePresented = pagePresented,
              directTouchInteraction = directTouchInteraction,
              platform = platform,
              selectionHandlesHidden = selectionHandlesHidden,
            )
            ?.firstOrNull { it.type == type } ?: return null
        return resolveSelectionHandleOverlayGeometry(placement, density, platform, image)
      }
      key(type) {
        // Keep handles in the editor's drawing layer for scrolling and toolbar backdrops.
        Canvas(
          modifier =
            Modifier.absoluteOffset {
                val topLeft = geometry()?.touchTargetTopLeft ?: Offset.Zero
                IntOffset(topLeft.x.roundToInt(), topLeft.y.roundToInt())
              }
              .layout { measurable, _ ->
                val size = geometry()?.touchTargetSize ?: Size.Zero
                val placeable =
                  measurable.measure(
                    Constraints.fixed(size.width.roundToInt(), size.height.roundToInt())
                  )
                layout(placeable.width, placeable.height) { placeable.place(0, 0) }
              }
              .editorSelectionHandleGestureExclusion()
        ) {
          val geometry = geometry() ?: return@Canvas
          translate(
            left =
              geometry.paintTopLeftInTouchTarget.x + geometry.touchTargetTopLeft.x -
                geometry.touchTargetTopLeft.x.roundToInt(),
            top =
              geometry.paintTopLeftInTouchTarget.y + geometry.touchTargetTopLeft.y -
                geometry.touchTargetTopLeft.y.roundToInt(),
          ) {
            val centerX = geometry.radiusPx
            if (platform == Platform.Android && image != null) {
              drawImage(image, colorFilter = ColorFilter.tint(color))
            } else if (platform == Platform.Android) {
              val radius = geometry.radiusPx
              val center = Offset(radius, radius)
              // The square corner attaches to the selection boundary; the cursor handle points up.
              rotate(if (type == EditorSelectionHandleType.Cursor) 45f else 0f, pivot = center) {
                drawCircle(color, radius, center)
                drawRect(
                  color,
                  topLeft = Offset(if (type == EditorSelectionHandleType.From) radius else 0f, 0f),
                  size = Size(radius, radius),
                )
              }
            } else if (type == EditorSelectionHandleType.From) {
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
  }
}

internal fun resolveSelectionHandleOverlayPlacements(
  state: EditorState,
  uiState: EditorUiState,
  editorRectInOverlay: Rect,
  density: Float,
  pagePresented: (Int) -> Boolean = { true },
  directTouchInteraction: Boolean,
  platform: Platform = Platform.Desktop,
  selectionHandlesHidden: Boolean = false,
): List<EditorSelectionHandleOverlayPlacement>? {
  if (
    !directTouchInteraction ||
      density <= 0f ||
      resolveTableCellSelections(state).isNotEmpty() ||
      selectionHandlesHidden
  ) {
    return null
  }
  val transform = uiState.resolveViewportTransform(pageSizes = state.pageSizes)
  if (state.selection.isCollapsed()) {
    if (
      platform != Platform.Android || !uiState.cursorHandle.isVisibleFor(state) || !uiState.focused
    )
      return null
    val cursor = state.cursor ?: return null
    if (!pagePresented(cursor.pageIdx)) return null
    return resolveSelectionHandleOverlayPlacement(
        type = EditorSelectionHandleType.Cursor,
        endpoint =
          PageRect(
            cursor.pageIdx,
            co.typie.editor.ffi.Rect(cursor.caret.x, cursor.caret.y, 0f, cursor.caret.height),
          ),
        transform = transform,
        editorRectInOverlay = editorRectInOverlay,
        density = density,
      )
      ?.let { listOf(it) }
  }
  val endpoints = state.selectionEndpoints ?: return null
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
  platform: Platform = Platform.Desktop,
  image: ImageBitmap? = null,
) =
  resolveSelectionHandleGeometry(
    type = placement.type,
    endpointTopLeftInOverlay = placement.endpointTopLeftInOverlay,
    stemHeightPx = placement.stemHeightPx,
    radiusPx =
      when {
        platform != Platform.Android -> EditorSelectionHandleRadiusDp
        placement.type == EditorSelectionHandleType.Cursor -> EditorAndroidCursorHandleRadiusDp
        else -> EditorAndroidSelectionHandleRadiusDp
      } * density,
    stemWidthPx = EditorSelectionHandleStemWidthDp * density,
    touchTargetPx = EditorSelectionHandleTouchTargetDp * density,
    platform = platform,
    image = image,
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
