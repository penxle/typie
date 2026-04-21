package co.typie.ui.component.sheet

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class SheetNestedScrollTest {

  @Test
  fun preScrollIsConsumedWhileSheetIsBelowTopStop() {
    assertTrue(
      shouldSheetConsumeNestedPreScroll(
        currentOffset = 420f,
        topStopOffset = 128f,
        availableY = -24f,
      )
    )
    assertFalse(
      shouldSheetConsumeNestedPreScroll(
        currentOffset = 128f,
        topStopOffset = 128f,
        availableY = -24f,
      )
    )
    assertFalse(
      shouldSheetConsumeNestedPreScroll(
        currentOffset = 420f,
        topStopOffset = 128f,
        availableY = 24f,
      )
    )
  }

  @Test
  fun partiallyCollapsedSheetSettlesEvenWithoutFlingVelocity() {
    assertTrue(shouldSheetSettleAfterNestedGesture(currentOffset = 172f, topStopOffset = 128f))
  }

  @Test
  fun fullyExpandedSheetDoesNotForceSettle() {
    assertFalse(shouldSheetSettleAfterNestedGesture(currentOffset = 128f, topStopOffset = 128f))
  }

  @Test
  fun postScrollConsumesOnlyDownwardLeftoverAtTopStop() {
    assertTrue(
      shouldSheetConsumeNestedPostScroll(
        currentOffset = 128f,
        topStopOffset = 128f,
        availableY = 18f,
      )
    )
    assertTrue(
      shouldSheetConsumeNestedPostScroll(
        currentOffset = 172f,
        topStopOffset = 128f,
        availableY = 18f,
      )
    )
    assertFalse(
      shouldSheetConsumeNestedPostScroll(
        currentOffset = 128f,
        topStopOffset = 128f,
        availableY = -18f,
      )
    )
  }

  @Test
  fun fastUpwardFlingTargetsTheNextHigherStop() {
    assertEquals(
      0,
      resolveSheetFlingTargetValue(
        anchors =
          listOf(
            SheetAnchor(value = 0, offset = 128f),
            SheetAnchor(value = 1, offset = 440f),
            SheetAnchor(value = -1, offset = 900f),
          ),
        currentOffset = 440f,
        velocity = -500f,
        velocityThreshold = 125f,
      ),
    )
  }

  @Test
  fun fastDownwardFlingTargetsTheNextLowerStopOrHiddenAnchor() {
    assertEquals(
      1,
      resolveSheetFlingTargetValue(
        anchors =
          listOf(
            SheetAnchor(value = 0, offset = 128f),
            SheetAnchor(value = 1, offset = 440f),
            SheetAnchor(value = -1, offset = 900f),
          ),
        currentOffset = 128f,
        velocity = 500f,
        velocityThreshold = 125f,
      ),
    )

    assertEquals(
      -1,
      resolveSheetFlingTargetValue(
        anchors =
          listOf(SheetAnchor(value = 0, offset = 440f), SheetAnchor(value = -1, offset = 900f)),
        currentOffset = 440f,
        velocity = 500f,
        velocityThreshold = 125f,
      ),
    )
  }

  @Test
  fun weakFlingUsesHalfDistanceThreshold() {
    assertEquals(
      0,
      resolveSheetFlingTargetValue(
        anchors =
          listOf(
            SheetAnchor(value = 0, offset = 128f),
            SheetAnchor(value = 1, offset = 440f),
            SheetAnchor(value = -1, offset = 900f),
          ),
        currentOffset = 250f,
        velocity = -80f,
        velocityThreshold = 125f,
      ),
    )

    assertEquals(
      1,
      resolveSheetFlingTargetValue(
        anchors =
          listOf(
            SheetAnchor(value = 0, offset = 128f),
            SheetAnchor(value = 1, offset = 440f),
            SheetAnchor(value = -1, offset = 900f),
          ),
        currentOffset = 320f,
        velocity = -80f,
        velocityThreshold = 125f,
      ),
    )
  }
}
