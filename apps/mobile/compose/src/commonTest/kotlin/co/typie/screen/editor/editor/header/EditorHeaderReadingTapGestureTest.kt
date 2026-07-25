package co.typie.screen.editor.editor.header

import androidx.compose.ui.geometry.Offset
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class EditorHeaderReadingTapGestureTest {
  @Test
  fun firstTapSchedulesHintAfterConsecutiveTapWindow() {
    val tracker = tracker()

    assertEquals(
      EditorHeaderReadingTapResult.None,
      tracker.onDown(position = Offset(10f, 10f), offset = 2, timeMillis = 0),
    )
    assertEquals(
      EditorHeaderReadingTapResult.None,
      tracker.onUp(position = Offset(10f, 10f), offset = 2, timeMillis = 20),
    )

    assertEquals(320, tracker.pendingConfirmationAtMillis)
    assertEquals(EditorHeaderReadingTapResult.None, tracker.confirm(timeMillis = 319))
    assertEquals(EditorHeaderReadingTapResult.ShowHint, tracker.confirm(timeMillis = 320))
    assertNull(tracker.pendingConfirmationAtMillis)
  }

  @Test
  fun secondTapWithinTimeAndDistanceActivatesTouchedOffset() {
    val tracker = tracker()
    tracker.onDown(position = Offset.Zero, offset = 0, timeMillis = 0)
    tracker.onUp(position = Offset.Zero, offset = 0, timeMillis = 10)

    assertEquals(
      EditorHeaderReadingTapResult.Activate(offset = 4),
      tracker.onDown(position = Offset(8f, 0f), offset = 4, timeMillis = 250),
    )
    assertNull(tracker.pendingConfirmationAtMillis)
  }

  @Test
  fun secondTapOutsideWindowStartsNewFirstTap() {
    val tracker = tracker()
    tracker.onDown(position = Offset.Zero, offset = 0, timeMillis = 0)
    tracker.onUp(position = Offset.Zero, offset = 0, timeMillis = 10)

    assertEquals(
      EditorHeaderReadingTapResult.None,
      tracker.onDown(position = Offset.Zero, offset = 4, timeMillis = 311),
    )
    assertNull(tracker.pendingConfirmationAtMillis)
    tracker.onUp(position = Offset.Zero, offset = 4, timeMillis = 320)
    assertEquals(620, tracker.pendingConfirmationAtMillis)
  }

  @Test
  fun movementLongPressCancelAndModeChangeClearPendingState() {
    val tracker = tracker()

    tracker.onDown(position = Offset.Zero, offset = 0, timeMillis = 0)
    tracker.onMove(position = Offset(9f, 0f))
    tracker.onUp(position = Offset(9f, 0f), offset = 1, timeMillis = 10)
    assertNull(tracker.pendingConfirmationAtMillis)

    tracker.onDown(position = Offset.Zero, offset = 0, timeMillis = 20)
    tracker.onLongPress()
    tracker.onUp(position = Offset.Zero, offset = 0, timeMillis = 30)
    assertNull(tracker.pendingConfirmationAtMillis)

    scheduleFirstTap(tracker, startMillis = 40)
    tracker.cancel()
    assertNull(tracker.pendingConfirmationAtMillis)

    scheduleFirstTap(tracker, startMillis = 80)
    tracker.onModeChanged()
    assertNull(tracker.pendingConfirmationAtMillis)
  }

  @Test
  fun settingDisabledFirstCompletedTapActivates() {
    val tracker = tracker(doubleTapToEditEnabled = false)
    tracker.onDown(position = Offset.Zero, offset = 0, timeMillis = 0)

    assertEquals(
      EditorHeaderReadingTapResult.Activate(offset = 3),
      tracker.onUp(position = Offset.Zero, offset = 3, timeMillis = 10),
    )
    assertNull(tracker.pendingConfirmationAtMillis)
  }

  @Test
  fun activationDisabledKeepsSingleTapAsSelectionWhenEditPreferenceIsDisabled() {
    val tracker = tracker(doubleTapToEditEnabled = false, editingActivationEnabled = false)
    tracker.onDown(position = Offset.Zero, offset = 0, timeMillis = 0)

    assertEquals(
      EditorHeaderReadingTapResult.None,
      tracker.onUp(position = Offset.Zero, offset = 3, timeMillis = 10),
    )
    assertEquals(EditorHeaderReadingTapResult.ShowHint, tracker.confirm(timeMillis = 310))
  }

  @Test
  fun activationDisabledLeavesConsecutiveTapNativeAndSuppressesTheHint() {
    val tracker = tracker(editingActivationEnabled = false)
    tracker.onDown(position = Offset.Zero, offset = 0, timeMillis = 0)
    tracker.onUp(position = Offset.Zero, offset = 0, timeMillis = 10)

    assertEquals(
      EditorHeaderReadingTapResult.None,
      tracker.onDown(position = Offset(8f, 0f), offset = 4, timeMillis = 250),
    )
    assertEquals(
      EditorHeaderReadingTapResult.None,
      tracker.onUp(position = Offset(8f, 0f), offset = 4, timeMillis = 260),
    )
    assertEquals(EditorHeaderReadingTapResult.None, tracker.confirm(timeMillis = 560))
    assertNull(tracker.pendingConfirmationAtMillis)
  }

  @Test
  fun confirmedSingleTapSkipsHintWhenItHandledExistingSelectionUi() {
    val tracker = tracker()
    tracker.onDown(position = Offset.Zero, offset = 0, timeMillis = 0)
    tracker.onUp(position = Offset.Zero, offset = 0, timeMillis = 10, suppressHint = true)

    assertEquals(EditorHeaderReadingTapResult.None, tracker.confirm(timeMillis = 310))
    assertNull(tracker.pendingConfirmationAtMillis)
  }

  private fun tracker(
    doubleTapToEditEnabled: Boolean = true,
    editingActivationEnabled: Boolean = true,
  ): EditorHeaderReadingTapTracker =
    EditorHeaderReadingTapTracker(
      touchSlopPx = 8f,
      maxTapDistancePx = 20f,
      doubleTapToEditEnabled = doubleTapToEditEnabled,
      editingActivationEnabled = editingActivationEnabled,
    )

  private fun scheduleFirstTap(tracker: EditorHeaderReadingTapTracker, startMillis: Long) {
    tracker.onDown(position = Offset.Zero, offset = 0, timeMillis = startMillis)
    tracker.onUp(position = Offset.Zero, offset = 0, timeMillis = startMillis + 10)
  }
}
