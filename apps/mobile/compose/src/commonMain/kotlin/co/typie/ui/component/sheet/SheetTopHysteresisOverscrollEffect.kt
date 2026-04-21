package co.typie.ui.component.sheet

import androidx.compose.foundation.OverscrollEffect
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.unit.Velocity
import kotlin.math.min

internal class SheetTopHysteresisOverscrollEffect : OverscrollEffect {
  private var pendingTopOverdragPx = 0f

  override fun applyToScroll(
    delta: Offset,
    source: NestedScrollSource,
    performScroll: (Offset) -> Offset,
  ): Offset {
    var availableY = delta.y
    var hysteresisConsumedY = 0f

    if (pendingTopOverdragPx > 0f && availableY > 0f) {
      val releasedY = min(availableY, pendingTopOverdragPx)
      pendingTopOverdragPx -= releasedY
      availableY -= releasedY
      hysteresisConsumedY += releasedY
    }

    val sheetConsumed = performScroll(Offset(delta.x, availableY))
    val leftoverY = availableY - sheetConsumed.y

    if (leftoverY < 0f) {
      pendingTopOverdragPx += -leftoverY
      hysteresisConsumedY += -leftoverY
    }

    return Offset(sheetConsumed.x, sheetConsumed.y + hysteresisConsumedY)
  }

  override suspend fun applyToFling(
    velocity: Velocity,
    performFling: suspend (Velocity) -> Velocity,
  ) {
    val forwardedVelocity = if (pendingTopOverdragPx > 0f) Velocity.Zero else velocity
    pendingTopOverdragPx = 0f
    performFling(forwardedVelocity)
  }

  override val isInProgress: Boolean
    get() = pendingTopOverdragPx > 0f
}
