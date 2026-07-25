package co.typie.editor.interaction.gestures

import androidx.compose.ui.geometry.Offset
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class EditorTapGestureTest {
  @Test
  fun `tap timer dispatch still completes the tap on pointer up`() {
    val gesture = createTapGesture()

    gesture.startPendingTap(pointerId = 1L, position = Offset(10f, 20f))

    assertTrue(gesture.canDispatchTapTimer)
    gesture.markTapDispatched()
    assertFalse(gesture.canDispatchTapTimer)
    assertEquals(
      EditorCompletedTap(count = 1, contentAlreadyDispatched = true, recordInHistory = true),
      gesture.onPointerUp(pointerId = 1L, position = Offset(10f, 20f), nowMillis = 160L),
    )
  }

  @Test
  fun `tap timer selection hit advances click count only after pointer up`() {
    val gesture = createTapGesture()

    gesture.startPendingTap(pointerId = 1L, position = Offset.Zero)
    gesture.markTapDispatched()

    assertEquals(
      1,
      gesture.onPointerUp(pointerId = 1L, position = Offset.Zero, nowMillis = 160L)?.count,
    )

    gesture.startPendingTap(pointerId = 2L, position = Offset.Zero)

    assertEquals(
      2,
      gesture.onPointerUp(pointerId = 2L, position = Offset.Zero, nowMillis = 240L)?.count,
    )
  }

  @Test
  fun `tap timer range selection keeps tap available for pointer up dispatch`() {
    val gesture = createTapGesture()

    gesture.startPendingTap(pointerId = 1L, position = Offset.Zero)

    assertTrue(gesture.canDispatchTapTimer)
    assertEquals(
      EditorCompletedTap(count = 1, contentAlreadyDispatched = false, recordInHistory = true),
      gesture.onPointerUp(pointerId = 1L, position = Offset.Zero, nowMillis = 160L),
    )
  }

  @Test
  fun `consecutive tap inside configured window dispatches count two`() {
    val gesture = createTapGesture()

    gesture.startPendingTap(pointerId = 1L, position = Offset(10f, 20f))
    val firstClick =
      gesture.onPointerUp(pointerId = 1L, position = Offset(10f, 20f), nowMillis = 40L)
    assertEquals(1, firstClick?.count)

    assertEquals(2, gesture.nextTapCount(position = Offset(18f, 26f), nowMillis = 240L))
  }

  @Test
  fun `consecutive third tap dispatches count three`() {
    val gesture = createTapGesture()

    gesture.recordTap(nowMillis = 40L, position = Offset(10f, 20f), clickCount = 1)
    gesture.recordTap(nowMillis = 240L, position = Offset(18f, 26f), clickCount = 2)

    gesture.startPendingTap(pointerId = 3L, position = Offset(20f, 28f), nowMillis = 390L)

    assertEquals(
      3,
      gesture.onPointerUp(pointerId = 3L, position = Offset(20f, 28f), nowMillis = 430L)?.count,
    )
  }

  @Test
  fun `committed triple tap resets the next tap to count one`() {
    val gesture = createTapGesture()

    gesture.recordTap(nowMillis = 40L, position = Offset(10f, 20f), clickCount = 1)
    gesture.recordTap(nowMillis = 240L, position = Offset(18f, 26f), clickCount = 2)
    gesture.recordTap(nowMillis = 390L, position = Offset(20f, 28f), clickCount = 3)

    assertEquals(1, gesture.nextTapCount(position = Offset(22f, 30f), nowMillis = 520L))
  }

  @Test
  fun `tap outside configured window resets click count`() {
    val gesture = createTapGesture()

    gesture.recordTap(nowMillis = 40L, position = Offset(10f, 20f), clickCount = 1)

    gesture.startPendingTap(pointerId = 2L, position = Offset(10f, 20f), nowMillis = 430L)

    assertEquals(
      1,
      gesture.onPointerUp(pointerId = 2L, position = Offset(10f, 20f), nowMillis = 470L)?.count,
    )
  }

  @Test
  fun `moving inside tap slop keeps pending tap dispatch`() {
    val gesture = createTapGesture()

    gesture.startPendingTap(pointerId = 1L, position = Offset.Zero)

    assertFalse(gesture.onPointerMove(pointerId = 1L, position = Offset(4f, 0f)))
    assertEquals(
      1,
      gesture.onPointerUp(pointerId = 1L, position = Offset(4f, 0f), nowMillis = 160L)?.count,
    )
  }

  @Test
  fun `moving beyond tap slop cancels pending tap without starting selection drag`() {
    val gesture = createTapGesture()

    gesture.startPendingTap(pointerId = 1L, position = Offset.Zero)

    assertTrue(gesture.onPointerMove(pointerId = 1L, position = Offset(9f, 0f)))
    assertFalse(gesture.canDispatchTapTimer)
    assertNull(gesture.onPointerUp(pointerId = 1L, position = Offset(9f, 0f), nowMillis = 160L))
  }

  @Test
  fun `plain drag does not advance consecutive tap count`() {
    val gesture = createTapGesture()

    gesture.recordTap(nowMillis = 100L, position = Offset.Zero, clickCount = 1)
    gesture.startPendingTap(pointerId = 2L, position = Offset.Zero, nowMillis = 200L)

    assertTrue(gesture.onPointerMove(pointerId = 2L, position = Offset(9f, 0f)))

    gesture.onPointerUp(pointerId = 2L, position = Offset(9f, 0f), nowMillis = 560L)
    gesture.startPendingTap(pointerId = 3L, position = Offset.Zero, nowMillis = 660L)

    assertEquals(
      1,
      gesture.onPointerUp(pointerId = 3L, position = Offset.Zero, nowMillis = 700L)?.count,
    )
  }

  @Test
  fun `cancelling active stream clears only the active tap candidate`() {
    val gesture = createTapGesture()

    gesture.startPendingTap(pointerId = 1L, position = Offset.Zero)
    assertTrue(gesture.cancelActivePointerStream())

    assertFalse(gesture.canDispatchTapTimer)
    assertFalse(gesture.cancelActivePointerStream())
  }

  @Test
  fun `editing promotion completes as a fresh tap without entering tap history`() {
    val gesture = createTapGesture()

    gesture.recordTap(nowMillis = 40L, position = Offset.Zero, clickCount = 1)
    gesture.startPendingTap(pointerId = 2L, position = Offset.Zero, nowMillis = 120L)
    gesture.prepareActiveTapForEditingPromotion()

    assertEquals(
      EditorCompletedTap(count = 1, contentAlreadyDispatched = true, recordInHistory = false),
      gesture.onPointerUp(pointerId = 2L, position = Offset.Zero, nowMillis = 160L),
    )
    assertEquals(2, gesture.nextTapCount(position = Offset.Zero, nowMillis = 200L))

    gesture.clearTapHistory()
    assertEquals(1, gesture.nextTapCount(position = Offset.Zero, nowMillis = 200L))
  }

  private fun EditorTapGesture.startPendingTap(
    pointerId: Long,
    position: Offset,
    nowMillis: Long = 0L,
  ) {
    startActivePointer(pointerId = pointerId, position = position, nowMillis = nowMillis)
  }

  private fun createTapGesture(): EditorTapGesture =
    EditorTapGesture(tapSlopPx = 8f, densityProvider = { 1f })
}
