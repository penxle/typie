package co.typie.screen.editor.editor.toolbar

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull

class ToolbarPageMetricsTest {
  @Test
  fun forwardHardStopUsesGestureStartPositionNotCurrentPosition() {
    val result =
      metrics.applyHardStop(
        currentPosition = TextEnd - Epsilon + 1f,
        proposedPosition = TextEnd + 12f,
        hardStop = null,
        gestureStartPosition = TextStart,
        activationEpsilon = Epsilon,
      )

    assertEquals(TextEnd, result.position)
    assertEquals(12f, result.rejectedDelta)
    assertNotNull(result.hardStop)
  }

  @Test
  fun forwardEscapeIsAllowedWhenGestureStartsNearScrollEnd() {
    val result =
      metrics.applyHardStop(
        currentPosition = TextEnd - 1f,
        proposedPosition = TextEnd + 12f,
        hardStop = null,
        gestureStartPosition = TextEnd,
        activationEpsilon = Epsilon,
      )

    assertEquals(TextEnd + 12f, result.position)
    assertEquals(0f, result.rejectedDelta)
    assertNull(result.hardStop)
  }

  @Test
  fun forwardEscapeIsAllowedEvenAfterDraggingAwayFromScrollEndInSameGesture() {
    val result =
      metrics.applyHardStop(
        currentPosition = TextEnd - Epsilon - 10f,
        proposedPosition = TextEnd + 12f,
        hardStop = null,
        gestureStartPosition = TextEnd,
        activationEpsilon = Epsilon,
      )

    assertEquals(TextEnd + 12f, result.position)
    assertEquals(0f, result.rejectedDelta)
    assertNull(result.hardStop)
  }

  @Test
  fun backwardHardStopUsesGestureStartPositionNotCurrentPosition() {
    val result =
      metrics.applyHardStop(
        currentPosition = TextStart + Epsilon - 1f,
        proposedPosition = TextStart - 12f,
        hardStop = null,
        gestureStartPosition = TextEnd,
        activationEpsilon = Epsilon,
      )

    assertEquals(TextStart, result.position)
    assertEquals(-12f, result.rejectedDelta)
    assertNotNull(result.hardStop)
  }

  @Test
  fun backwardEscapeIsAllowedWhenGestureStartsNearScrollStart() {
    val result =
      metrics.applyHardStop(
        currentPosition = TextStart + 1f,
        proposedPosition = TextStart - 12f,
        hardStop = null,
        gestureStartPosition = TextStart,
        activationEpsilon = Epsilon,
      )

    assertEquals(TextStart - 12f, result.position)
    assertEquals(0f, result.rejectedDelta)
    assertNull(result.hardStop)
  }

  @Test
  fun backwardEscapeIsAllowedEvenAfterDraggingAwayFromScrollStartInSameGesture() {
    val result =
      metrics.applyHardStop(
        currentPosition = TextStart + Epsilon + 10f,
        proposedPosition = TextStart - 12f,
        hardStop = null,
        gestureStartPosition = TextStart,
        activationEpsilon = Epsilon,
      )

    assertEquals(TextStart - 12f, result.position)
    assertEquals(0f, result.rejectedDelta)
    assertNull(result.hardStop)
  }

  @Test
  fun internalOverflowFlingDecaysInsideTextRange() {
    assertEquals(
      true,
      metrics.decaysFlingWithinInternalScroll(position = TextStart + 12f, velocity = -800f),
    )
  }

  @Test
  fun pageTransitionFlingDoesNotDecayBeforeSnap() {
    assertEquals(
      false,
      metrics.decaysFlingWithinInternalScroll(position = TextStart - 12f, velocity = -800f),
    )
  }

  @Test
  fun overflowStartFlingDecaysWhenVelocityMovesIntoOverflow() {
    assertEquals(
      true,
      metrics.decaysFlingWithinInternalScroll(position = TextStart, velocity = -800f),
    )
  }

  private companion object {
    const val PageDistance = 100f
    const val TextStart = PageDistance
    const val TextRange = 40f
    const val TextEnd = TextStart + TextRange
    const val Epsilon = 10f

    val metrics =
      ToolbarPagerMetrics(
        pageDistance = PageDistance,
        scrollRanges = listOf(0, TextRange.toInt(), 0),
      )
  }
}
