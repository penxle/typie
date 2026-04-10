package co.typie.ui.component.popover

import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntRect
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.window.PopupPositionProvider

enum class PopoverAlign {
  Start,
  Center,
  End,
}

enum class PopoverSide {
  Below,
  Above,
}

data class PopoverPlacement(val side: PopoverSide, val align: PopoverAlign) {
  companion object {
    val BelowStart = PopoverPlacement(side = PopoverSide.Below, align = PopoverAlign.Start)
    val BelowCenter = PopoverPlacement(side = PopoverSide.Below, align = PopoverAlign.Center)
    val BelowEnd = PopoverPlacement(side = PopoverSide.Below, align = PopoverAlign.End)
    val AboveStart = PopoverPlacement(side = PopoverSide.Above, align = PopoverAlign.Start)
    val AboveCenter = PopoverPlacement(side = PopoverSide.Above, align = PopoverAlign.Center)
    val AboveEnd = PopoverPlacement(side = PopoverSide.Above, align = PopoverAlign.End)
  }
}

internal class PopoverPlacementProvider(
  private val placement: PopoverPlacement,
  private val screenPadding: PopoverScreenPadding,
) : PopupPositionProvider {

  override fun calculatePosition(
    anchorBounds: IntRect,
    windowSize: IntSize,
    layoutDirection: LayoutDirection,
    popupContentSize: IntSize,
  ): IntOffset {
    val showBelow =
      shouldShowBelow(
        placement = placement,
        childHeight = popupContentSize.height,
        windowHeight = windowSize.height,
        anchorRect = anchorBounds,
        screenPadding = screenPadding,
      )

    val unclampedX =
      when (placement.align) {
        PopoverAlign.Start -> anchorBounds.left
        PopoverAlign.Center -> anchorBounds.left + (anchorBounds.width - popupContentSize.width) / 2
        PopoverAlign.End -> anchorBounds.right - popupContentSize.width
      }

    val unclampedY =
      if (showBelow) anchorBounds.top else anchorBounds.bottom - popupContentSize.height

    val minX =
      when (placement.align) {
        PopoverAlign.Start -> anchorBounds.left
        else -> screenPadding.left
      }
    val maxX =
      when (placement.align) {
        PopoverAlign.End -> anchorBounds.right - popupContentSize.width
        else -> windowSize.width - screenPadding.right - popupContentSize.width
      }
    val minY = if (showBelow) anchorBounds.top else screenPadding.top
    val maxY =
      if (showBelow) {
        windowSize.height - screenPadding.bottom - popupContentSize.height
      } else {
        anchorBounds.bottom - popupContentSize.height
      }

    return IntOffset(x = clamp(unclampedX, minX, maxX), y = clamp(unclampedY, minY, maxY))
  }
}

internal fun shouldShowBelow(
  placement: PopoverPlacement,
  childHeight: Int,
  windowHeight: Int,
  anchorRect: IntRect,
  screenPadding: PopoverScreenPadding,
): Boolean {
  val bottomSpace = windowHeight - screenPadding.bottom - anchorRect.top
  val topSpace = anchorRect.bottom - screenPadding.top
  val prefersBottom = placement.side == PopoverSide.Below

  if (prefersBottom) {
    if (childHeight <= bottomSpace) return true
    if (childHeight <= topSpace) return false
    return bottomSpace >= topSpace
  }

  if (childHeight <= topSpace) return false
  if (childHeight <= bottomSpace) return true
  return bottomSpace > topSpace
}

internal fun resolvedPlacement(placement: PopoverPlacement, showBelow: Boolean): PopoverPlacement {
  return placement.copy(side = if (showBelow) PopoverSide.Below else PopoverSide.Above)
}

private fun clamp(value: Int, min: Int, max: Int): Int {
  if (max < min) return min
  return value.coerceIn(min, max)
}
