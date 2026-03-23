package co.typie.ui.component

import kotlin.test.Test
import kotlin.test.assertEquals

class TooltipScrubGestureTest {

  @Test
  fun resolveTooltipGestureAction_startsScrubBeforeVelocityThreshold() {
    val result = resolveTooltipGestureAction(
      phase = TooltipGesturePhase.Tooltip,
      velocityX = 120f,
      velocityY = 80f,
    )

    assertEquals(TooltipGestureAction.BeginScrub, result)
  }

  @Test
  fun resolveTooltipGestureAction_switchesToHorizontalScrollWhenFast() {
    val result = resolveTooltipGestureAction(
      phase = TooltipGesturePhase.Scrub,
      velocityX = 900f,
      velocityY = 240f,
    )

    assertEquals(TooltipGestureAction.BeginHorizontalScroll, result)
  }

  @Test
  fun resolveTooltipGestureAction_switchesToVerticalScrollWhenFast() {
    val result = resolveTooltipGestureAction(
      phase = TooltipGesturePhase.Tooltip,
      velocityX = 180f,
      velocityY = 840f,
    )

    assertEquals(TooltipGestureAction.BeginVerticalScroll, result)
  }
}
