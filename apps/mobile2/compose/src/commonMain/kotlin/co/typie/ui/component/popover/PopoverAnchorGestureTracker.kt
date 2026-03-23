package co.typie.ui.component.popover

import androidx.compose.ui.geometry.Offset

internal data class AnchorPointerUpdate(
  val pointerState: AnchorPointerState,
  val consumeChange: Boolean,
)

internal class PopoverAnchorGestureTracker(
  private val origin: Offset,
  private val armDistancePx: Float,
) {
  private var isSelectionArmed = false

  fun start(): AnchorPointerUpdate {
    return AnchorPointerUpdate(
      pointerState = AnchorPointerState(
        position = origin,
        isSelectionArmed = false,
        isUp = false,
      ),
      consumeChange = true,
    )
  }

  fun update(
    currentPosition: Offset,
    elapsedMillis: Long,
    isPressed: Boolean,
  ): AnchorPointerUpdate {
    val distance = (currentPosition - origin).getDistance()
    if (!isSelectionArmed && elapsedMillis >= PopoverDefaults.ArmDelayMs && distance > armDistancePx) {
      isSelectionArmed = true
    }

    return AnchorPointerUpdate(
      pointerState = AnchorPointerState(
        position = currentPosition,
        isSelectionArmed = isSelectionArmed,
        isUp = !isPressed,
      ),
      consumeChange = true,
    )
  }
}
