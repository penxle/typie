package co.typie.editor.interaction.gestures

import androidx.compose.ui.geometry.Offset
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class EditorTapGestureTest {
  @Test
  fun `tap dispatch delay includes legacy tap down deadline before tap timer`() {
    assertEquals(250L, EditorTapDispatchDelayMillis)
  }

  @Test
  fun `tap timer can dispatch a primary click once`() {
    val gesture = EditorTapGesture(tapSlopPx = 8f)

    gesture.startPendingTap(pointerId = 1L, position = Offset(10f, 20f))

    assertTrue(gesture.canDispatchTapTimer)
    gesture.markTapDispatched()
    assertFalse(gesture.canDispatchTapTimer)
    assertNull(gesture.onPointerUp(pointerId = 1L, position = Offset(10f, 20f), nowMillis = 160L))
  }

  @Test
  fun `tap timer selection hit consumes pending tap without advancing click count`() {
    val gesture = EditorTapGesture(tapSlopPx = 8f)

    gesture.startPendingTap(pointerId = 1L, position = Offset.Zero)
    gesture.markTapDispatched()

    assertNull(gesture.onPointerUp(pointerId = 1L, position = Offset.Zero, nowMillis = 160L))

    gesture.startPendingTap(pointerId = 2L, position = Offset.Zero)

    assertEquals(1, gesture.onPointerUp(pointerId = 2L, position = Offset.Zero, nowMillis = 240L))
  }

  @Test
  fun `tap timer range selection keeps tap available for pointer up dispatch`() {
    val gesture = EditorTapGesture(tapSlopPx = 8f)

    gesture.startPendingTap(pointerId = 1L, position = Offset.Zero)

    assertTrue(gesture.canDispatchTapTimer)
    assertEquals(1, gesture.onPointerUp(pointerId = 1L, position = Offset.Zero, nowMillis = 160L))
  }

  @Test
  fun `consecutive tap inside configured window dispatches count two`() {
    val gesture = EditorTapGesture(tapSlopPx = 8f)

    gesture.startPendingTap(pointerId = 1L, position = Offset(10f, 20f))
    val firstClick =
      gesture.onPointerUp(pointerId = 1L, position = Offset(10f, 20f), nowMillis = 40L)
    assertEquals(1, firstClick)
    gesture.recordTap(nowMillis = 40L, position = Offset(10f, 20f), clickCount = firstClick!!)

    assertEquals(2, gesture.nextTapCount(position = Offset(18f, 26f), nowMillis = 240L))
  }

  @Test
  fun `double tap clears tap history so third tap dispatches count one`() {
    val gesture = EditorTapGesture(tapSlopPx = 8f)

    gesture.recordTap(nowMillis = 40L, position = Offset(10f, 20f), clickCount = 1)
    gesture.recordTap(nowMillis = 240L, position = Offset(18f, 26f), clickCount = 2)

    gesture.startPendingTap(pointerId = 3L, position = Offset(20f, 28f))

    assertEquals(
      1,
      gesture.onPointerUp(pointerId = 3L, position = Offset(20f, 28f), nowMillis = 390L),
    )
  }

  @Test
  fun `consecutive taps after double tap can form a new double tap`() {
    val gesture = EditorTapGesture(tapSlopPx = 8f)

    gesture.recordTap(nowMillis = 40L, position = Offset(10f, 20f), clickCount = 1)
    gesture.recordTap(nowMillis = 240L, position = Offset(18f, 26f), clickCount = 2)
    gesture.recordTap(nowMillis = 390L, position = Offset(20f, 28f), clickCount = 1)

    assertEquals(2, gesture.nextTapCount(position = Offset(22f, 30f), nowMillis = 520L))
  }

  @Test
  fun `tap outside configured window resets click count`() {
    val gesture = EditorTapGesture(tapSlopPx = 8f)

    gesture.recordTap(nowMillis = 40L, position = Offset(10f, 20f), clickCount = 1)

    gesture.startPendingTap(pointerId = 2L, position = Offset(10f, 20f))

    assertEquals(
      1,
      gesture.onPointerUp(pointerId = 2L, position = Offset(10f, 20f), nowMillis = 430L),
    )
  }

  @Test
  fun `moving inside tap slop keeps pending tap dispatch`() {
    val gesture = EditorTapGesture(tapSlopPx = 8f)

    gesture.startPendingTap(pointerId = 1L, position = Offset.Zero)

    assertFalse(gesture.onPointerMove(pointerId = 1L, position = Offset(4f, 0f)))
    assertEquals(
      1,
      gesture.onPointerUp(pointerId = 1L, position = Offset(4f, 0f), nowMillis = 160L),
    )
  }

  @Test
  fun `moving beyond tap slop cancels pending tap without starting selection drag`() {
    val gesture = EditorTapGesture(tapSlopPx = 8f)

    gesture.startPendingTap(pointerId = 1L, position = Offset.Zero)

    assertTrue(gesture.onPointerMove(pointerId = 1L, position = Offset(9f, 0f)))
    assertFalse(gesture.canDispatchTapTimer)
    assertNull(gesture.onPointerUp(pointerId = 1L, position = Offset(9f, 0f), nowMillis = 160L))
  }

  @Test
  fun `plain drag does not advance consecutive tap count`() {
    val gesture = EditorTapGesture(tapSlopPx = 8f)

    gesture.recordTap(nowMillis = 100L, position = Offset.Zero, clickCount = 1)
    gesture.startPendingTap(pointerId = 2L, position = Offset.Zero)

    assertTrue(gesture.onPointerMove(pointerId = 2L, position = Offset(9f, 0f)))

    gesture.onPointerUp(pointerId = 2L, position = Offset(9f, 0f), nowMillis = 560L)
    gesture.startPendingTap(pointerId = 3L, position = Offset.Zero)

    assertEquals(1, gesture.onPointerUp(pointerId = 3L, position = Offset.Zero, nowMillis = 700L))
  }

  @Test
  fun `second pointer cancels active tap and ignores until every pointer is up`() {
    val gesture = EditorTapGesture(tapSlopPx = 8f)

    gesture.startPendingTap(pointerId = 1L, position = Offset.Zero)
    gesture.addPressedPointer(2L)
    gesture.cancelActivePointerAndIgnoreUntilAllPointersUp()

    assertTrue(gesture.isIgnoringUntilAllPointersUp)
    assertFalse(gesture.canDispatchTapTimer)

    gesture.onPointerUp(pointerId = 1L, position = Offset.Zero, nowMillis = 160L)
    assertTrue(gesture.isIgnoringUntilAllPointersUp)

    gesture.onPointerUp(pointerId = 2L, position = Offset(4f, 4f), nowMillis = 170L)
    assertFalse(gesture.isIgnoringUntilAllPointersUp)
  }

  private fun EditorTapGesture.startPendingTap(pointerId: Long, position: Offset) {
    addPressedPointer(pointerId)
    startActivePointer(pointerId = pointerId, position = position)
    markTapPending()
  }
}
