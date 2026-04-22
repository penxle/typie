package co.typie.screen.editor.editor.zoom

import co.typie.editor.EditorViewportAnchor
import co.typie.editor.body.resolvePaginatedPageGap
import co.typie.editor.ffi.Size as PageSize
import co.typie.screen.editor.editor.state.EditorScreenState
import kotlin.math.roundToInt

internal data class EditorZoomViewportScrollTarget(
  val horizontalScroll: Float,
  val verticalScroll: Float,
)

internal fun resolveZoomViewportScrollTarget(
  anchor: EditorViewportAnchor,
  focalX: Float,
  focalY: Float,
  displayZoom: Float,
  currentHorizontalScroll: Float,
  currentVerticalScroll: Float,
  pageSizes: List<PageSize>,
): EditorZoomViewportScrollTarget? {
  if (anchor.page !in pageSizes.indices) {
    return null
  }

  val effectiveDisplayZoom =
    if (displayZoom.isFinite() && displayZoom > 0f) {
      displayZoom
    } else {
      1f
    }
  val anchorX = anchor.x * effectiveDisplayZoom
  val anchorY =
    resolveZoomedPageTop(
      page = anchor.page,
      pageSizes = pageSizes,
      displayZoom = effectiveDisplayZoom,
    ) + anchor.y * effectiveDisplayZoom

  return EditorZoomViewportScrollTarget(
    horizontalScroll = currentHorizontalScroll + anchorX - focalX,
    verticalScroll = currentVerticalScroll + anchorY - focalY,
  )
}

internal fun syncViewportToZoomAnchor(
  state: EditorScreenState,
  pageSizes: List<PageSize>,
  anchor: EditorViewportAnchor,
  focalX: Float,
  focalY: Float,
  displayZoom: Float,
  density: Float,
) {
  if (density <= 0f) {
    return
  }

  val currentHorizontalScroll = state.horizontalScrollState.value / density
  val currentVerticalScroll = state.scrollState.value / density
  val target =
    resolveZoomViewportScrollTarget(
      anchor = anchor,
      focalX = focalX,
      focalY = focalY,
      displayZoom = displayZoom,
      currentHorizontalScroll = currentHorizontalScroll,
      currentVerticalScroll = currentVerticalScroll,
      pageSizes = pageSizes,
    ) ?: return

  state.horizontalScrollState.dispatchToDp(target.horizontalScroll, density)
  state.scrollState.dispatchToDp(target.verticalScroll, density)
}

private fun resolveZoomedPageTop(page: Int, pageSizes: List<PageSize>, displayZoom: Float): Float {
  var top = 0f
  val pageGap = resolvePaginatedPageGap(displayZoom)
  repeat(page) { index ->
    top += pageSizes[index].height * displayZoom
    if (index < pageSizes.lastIndex) {
      top += pageGap
    }
  }
  return top
}

private fun androidx.compose.foundation.ScrollState.dispatchToDp(
  targetScroll: Float,
  density: Float,
) {
  if (density <= 0f) {
    return
  }

  val targetPx = (targetScroll * density).roundToInt().coerceIn(0, maxValue)
  val deltaPx = targetPx - value
  if (deltaPx == 0) {
    return
  }

  dispatchRawDelta(deltaPx.toFloat())
}
