package co.typie.ui.component.popover

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import co.typie.ext.toPx
import kotlin.math.roundToInt

data class PopoverTransitionFrame(val left: Dp, val top: Dp, val width: Dp, val height: Dp)

@Composable
fun rememberPopoverTransitionProgress(): Float {
  return (LocalPopoverPaneTransition.current?.progress ?: 1f).coerceIn(0f, 1f)
}

@Composable
fun PopoverTransitionElement(
  collapsedFrame: PopoverTransitionFrame,
  expandedFrame: PopoverTransitionFrame,
  modifier: Modifier = Modifier,
  panePadding: Dp = PopoverDefaults.PanePadding,
  collapsedContent: (@Composable BoxScope.() -> Unit)? = null,
  expandedContent: @Composable BoxScope.() -> Unit,
) {
  val density = LocalDensity.current
  val transition = LocalPopoverPaneTransition.current
  val progress = rememberPopoverTransitionProgress()
  val anchorContentRect = transition?.anchorContentRect

  val panePaddingPx = panePadding.toPx(density)
  val collapsedLeftPx =
    if (anchorContentRect == null) {
      expandedFrame.left.toPx(density)
    } else {
      anchorContentRect.left + collapsedFrame.left.toPx(density) - panePaddingPx
    }
  val collapsedTopPx =
    if (anchorContentRect == null) {
      expandedFrame.top.toPx(density)
    } else {
      anchorContentRect.top + collapsedFrame.top.toPx(density) - panePaddingPx
    }
  val expandedLeftPx = expandedFrame.left.toPx(density)
  val expandedTopPx = expandedFrame.top.toPx(density)
  val collapsedWidthPx = collapsedFrame.width.toPx(density)
  val collapsedHeightPx = collapsedFrame.height.toPx(density)
  val expandedWidthPx = expandedFrame.width.toPx(density)
  val expandedHeightPx = expandedFrame.height.toPx(density)

  val left = lerp(collapsedLeftPx, expandedLeftPx, progress)
  val top = lerp(collapsedTopPx, expandedTopPx, progress)
  val width = lerp(collapsedWidthPx, expandedWidthPx, progress)
  val height = lerp(collapsedHeightPx, expandedHeightPx, progress)

  Box(
    contentAlignment = Alignment.CenterStart,
    modifier =
      modifier
        .offset { IntOffset(x = left.roundToInt(), y = top.roundToInt()) }
        .size(width = with(density) { width.toDp() }, height = with(density) { height.toDp() }),
  ) {
    if (collapsedContent != null) {
      Box(modifier = Modifier.graphicsLayer { alpha = 1f - progress }) { collapsedContent() }
      Box(modifier = Modifier.graphicsLayer { alpha = progress }) { expandedContent() }
    } else {
      expandedContent()
    }
  }
}

private fun lerp(start: Float, end: Float, fraction: Float): Float {
  return start + (end - start) * fraction
}
