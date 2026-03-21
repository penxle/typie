package co.typie.ui.component.popover

import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntRect
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.window.PopupPositionProvider

enum class PopoverPosition {
  BottomLeft, BottomCenter, BottomRight,
  TopLeft, TopCenter, TopRight,
}

internal class PopoverPositionProvider(
  private val position: PopoverPosition,
  private val screenPadding: PopoverScreenPadding,
) : PopupPositionProvider {

  var lastShowBelow: Boolean = true
    private set

  override fun calculatePosition(
    anchorBounds: IntRect,
    windowSize: IntSize,
    layoutDirection: LayoutDirection,
    popupContentSize: IntSize,
  ): IntOffset {
    val showBelow = shouldShowBelow(
      position = position,
      childHeight = popupContentSize.height,
      windowHeight = windowSize.height,
      anchorRect = anchorBounds,
      screenPadding = screenPadding,
    )

    val unclampedX = when (position) {
      PopoverPosition.BottomLeft, PopoverPosition.TopLeft ->
        anchorBounds.left

      PopoverPosition.BottomCenter, PopoverPosition.TopCenter ->
        anchorBounds.left + (anchorBounds.width - popupContentSize.width) / 2

      PopoverPosition.BottomRight, PopoverPosition.TopRight ->
        anchorBounds.right - popupContentSize.width
    }

    val unclampedY =
      if (showBelow) anchorBounds.top else anchorBounds.bottom - popupContentSize.height

    val minX = when (position) {
      PopoverPosition.BottomLeft, PopoverPosition.TopLeft -> 0
      else -> screenPadding.left
    }
    val maxX = when (position) {
      PopoverPosition.BottomRight, PopoverPosition.TopRight ->
        windowSize.width - popupContentSize.width

      else ->
        windowSize.width - screenPadding.right - popupContentSize.width
    }
    val minY = if (showBelow) 0 else screenPadding.top
    val maxY = if (showBelow) {
      windowSize.height - screenPadding.bottom - popupContentSize.height
    } else {
      windowSize.height - popupContentSize.height
    }

    lastShowBelow = showBelow

    return IntOffset(
      x = clamp(unclampedX, minX, maxX),
      y = clamp(unclampedY, minY, maxY),
    )
  }
}

internal fun shouldShowBelow(
  position: PopoverPosition,
  childHeight: Int,
  windowHeight: Int,
  anchorRect: IntRect,
  screenPadding: PopoverScreenPadding,
): Boolean {
  val bottomSpace = windowHeight - screenPadding.bottom - anchorRect.top
  val topSpace = anchorRect.bottom - screenPadding.top
  val prefersBottom = position == PopoverPosition.BottomLeft ||
    position == PopoverPosition.BottomCenter ||
    position == PopoverPosition.BottomRight

  if (prefersBottom) {
    if (childHeight <= bottomSpace) return true
    if (childHeight <= topSpace) return false
    return bottomSpace >= topSpace
  }

  if (childHeight <= topSpace) return false
  if (childHeight <= bottomSpace) return true
  return bottomSpace > topSpace
}

internal fun effectivePosition(position: PopoverPosition, showBelow: Boolean): PopoverPosition {
  return when {
    position == PopoverPosition.BottomLeft && !showBelow -> PopoverPosition.TopLeft
    position == PopoverPosition.BottomCenter && !showBelow -> PopoverPosition.TopCenter
    position == PopoverPosition.BottomRight && !showBelow -> PopoverPosition.TopRight
    position == PopoverPosition.TopLeft && showBelow -> PopoverPosition.BottomLeft
    position == PopoverPosition.TopCenter && showBelow -> PopoverPosition.BottomCenter
    position == PopoverPosition.TopRight && showBelow -> PopoverPosition.BottomRight
    else -> position
  }
}

private fun clamp(value: Int, min: Int, max: Int): Int {
  if (max < min) return min
  return value.coerceIn(min, max)
}
