package co.typie.editor.gesture

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
  fun `tap timer dispatches a primary click once`() {
    val gesture = EditorTapGesture(tapSlopPx = 8f)

    gesture.onPointerDown(pointerId = 1L, position = Offset(10f, 20f), canStart = { true })

    assertEquals(
      EditorInteractionTapDispatch(position = Offset(10f, 20f), clickCount = 1),
      gesture.onTapTimer(nowMillis = 150L),
    )
    assertEquals(
      EditorInteractionPointerResult(consume = true),
      gesture.onPointerUp(
        pointerId = 1L,
        position = Offset(10f, 20f),
        nowMillis = 160L,
        canFinish = { true },
      ),
    )
  }

  @Test
  fun `tap timer selection hit consumes pending tap without advancing click count`() {
    val gesture = EditorTapGesture(tapSlopPx = 8f)

    gesture.onPointerDown(pointerId = 1L, position = Offset.Zero, canStart = { true })

    assertNull(gesture.onTapTimer(nowMillis = 150L, isSelectionHit = { true }))
    assertEquals(
      EditorInteractionPointerResult(consume = true),
      gesture.onPointerUp(
        pointerId = 1L,
        position = Offset.Zero,
        nowMillis = 160L,
        canFinish = { true },
      ),
    )

    gesture.onPointerDown(pointerId = 2L, position = Offset.Zero, canStart = { true })

    assertEquals(
      EditorInteractionTapDispatch(position = Offset.Zero, clickCount = 1),
      gesture
        .onPointerUp(pointerId = 2L, position = Offset.Zero, nowMillis = 240L, canFinish = { true })
        .tapDispatch,
    )
  }

  @Test
  fun `tap timer range selection keeps tap available for pointer up dispatch`() {
    val gesture = EditorTapGesture(tapSlopPx = 8f)

    gesture.onPointerDown(pointerId = 1L, position = Offset.Zero, canStart = { true })

    assertNull(gesture.onTapTimer(nowMillis = 150L, hasRangeSelection = { true }))
    assertEquals(
      EditorInteractionTapDispatch(position = Offset.Zero, clickCount = 1),
      gesture
        .onPointerUp(pointerId = 1L, position = Offset.Zero, nowMillis = 160L, canFinish = { true })
        .tapDispatch,
    )
  }

  @Test
  fun `consecutive tap inside configured window dispatches count two`() {
    val gesture = EditorTapGesture(tapSlopPx = 8f)

    gesture.onPointerDown(pointerId = 1L, position = Offset(10f, 20f), canStart = { true })
    assertEquals(
      EditorInteractionTapDispatch(position = Offset(10f, 20f), clickCount = 1),
      gesture
        .onPointerUp(
          pointerId = 1L,
          position = Offset(10f, 20f),
          nowMillis = 40L,
          canFinish = { true },
        )
        .tapDispatch,
    )

    gesture.onPointerDown(pointerId = 2L, position = Offset(18f, 26f), canStart = { true })

    assertEquals(
      EditorInteractionTapDispatch(position = Offset(18f, 26f), clickCount = 2),
      gesture
        .onPointerUp(
          pointerId = 2L,
          position = Offset(18f, 26f),
          nowMillis = 240L,
          canFinish = { true },
        )
        .tapDispatch,
    )
  }

  @Test
  fun `double tap clears tap history so third tap dispatches count one`() {
    val gesture = EditorTapGesture(tapSlopPx = 8f)

    gesture.onPointerDown(pointerId = 1L, position = Offset(10f, 20f), canStart = { true })
    gesture.onPointerUp(
      pointerId = 1L,
      position = Offset(10f, 20f),
      nowMillis = 40L,
      canFinish = { true },
    )

    gesture.onPointerDown(pointerId = 2L, position = Offset(18f, 26f), canStart = { true })
    gesture.onPointerUp(
      pointerId = 2L,
      position = Offset(18f, 26f),
      nowMillis = 240L,
      canFinish = { true },
    )

    gesture.onPointerDown(pointerId = 3L, position = Offset(20f, 28f), canStart = { true })

    assertEquals(
      EditorInteractionTapDispatch(position = Offset(20f, 28f), clickCount = 1),
      gesture
        .onPointerUp(
          pointerId = 3L,
          position = Offset(20f, 28f),
          nowMillis = 390L,
          canFinish = { true },
        )
        .tapDispatch,
    )
  }

  @Test
  fun `consecutive taps after double tap can form a new double tap`() {
    val gesture = EditorTapGesture(tapSlopPx = 8f)

    gesture.onPointerDown(pointerId = 1L, position = Offset(10f, 20f), canStart = { true })
    gesture.onPointerUp(
      pointerId = 1L,
      position = Offset(10f, 20f),
      nowMillis = 40L,
      canFinish = { true },
    )

    gesture.onPointerDown(pointerId = 2L, position = Offset(18f, 26f), canStart = { true })
    gesture.onPointerUp(
      pointerId = 2L,
      position = Offset(18f, 26f),
      nowMillis = 240L,
      canFinish = { true },
    )

    gesture.onPointerDown(pointerId = 3L, position = Offset(20f, 28f), canStart = { true })
    gesture.onPointerUp(
      pointerId = 3L,
      position = Offset(20f, 28f),
      nowMillis = 390L,
      canFinish = { true },
    )

    gesture.onPointerDown(pointerId = 4L, position = Offset(22f, 30f), canStart = { true })

    assertEquals(
      EditorInteractionTapDispatch(position = Offset(22f, 30f), clickCount = 2),
      gesture
        .onPointerUp(
          pointerId = 4L,
          position = Offset(22f, 30f),
          nowMillis = 520L,
          canFinish = { true },
        )
        .tapDispatch,
    )
  }

  @Test
  fun `tap outside configured window resets click count`() {
    val gesture = EditorTapGesture(tapSlopPx = 8f)

    gesture.onPointerDown(pointerId = 1L, position = Offset(10f, 20f), canStart = { true })
    gesture.onPointerUp(
      pointerId = 1L,
      position = Offset(10f, 20f),
      nowMillis = 40L,
      canFinish = { true },
    )

    gesture.onPointerDown(pointerId = 2L, position = Offset(10f, 20f), canStart = { true })

    assertEquals(
      EditorInteractionTapDispatch(position = Offset(10f, 20f), clickCount = 1),
      gesture
        .onPointerUp(
          pointerId = 2L,
          position = Offset(10f, 20f),
          nowMillis = 430L,
          canFinish = { true },
        )
        .tapDispatch,
    )
  }

  @Test
  fun `moving inside tap slop keeps pending tap dispatch`() {
    val gesture = EditorTapGesture(tapSlopPx = 8f)

    gesture.onPointerDown(pointerId = 1L, position = Offset.Zero, canStart = { true })

    assertEquals(
      EditorInteractionPointerResult(),
      gesture.onPointerMove(pointerId = 1L, position = Offset(4f, 0f)),
    )
    assertEquals(
      EditorInteractionTapDispatch(position = Offset(4f, 0f), clickCount = 1),
      gesture
        .onPointerUp(
          pointerId = 1L,
          position = Offset(4f, 0f),
          nowMillis = 160L,
          canFinish = { true },
        )
        .tapDispatch,
    )
  }

  @Test
  fun `moving beyond tap slop cancels pending tap without starting selection drag`() {
    val gesture = EditorTapGesture(tapSlopPx = 8f)

    gesture.onPointerDown(pointerId = 1L, position = Offset.Zero, canStart = { true })

    assertEquals(
      EditorInteractionPointerResult(cancelTapDispatch = true),
      gesture.onPointerMove(pointerId = 1L, position = Offset(9f, 0f)),
    )
    assertNull(gesture.onTapTimer(nowMillis = 150L))
    assertEquals(
      EditorInteractionPointerResult(),
      gesture.onPointerUp(
        pointerId = 1L,
        position = Offset(9f, 0f),
        nowMillis = 160L,
        canFinish = { true },
      ),
    )
  }

  @Test
  fun `plain drag does not advance consecutive tap count`() {
    val gesture = EditorTapGesture(tapSlopPx = 8f)

    gesture.onPointerDown(pointerId = 1L, position = Offset.Zero, canStart = { true })
    gesture.onPointerUp(
      pointerId = 1L,
      position = Offset.Zero,
      nowMillis = 100L,
      canFinish = { true },
    )
    gesture.onPointerDown(pointerId = 2L, position = Offset.Zero, canStart = { true })

    assertEquals(
      EditorInteractionPointerResult(cancelTapDispatch = true),
      gesture.onPointerMove(pointerId = 2L, position = Offset(9f, 0f)),
    )

    gesture.onPointerUp(
      pointerId = 2L,
      position = Offset(9f, 0f),
      nowMillis = 520L,
      canFinish = { true },
    )
    gesture.onPointerDown(pointerId = 3L, position = Offset.Zero, canStart = { true })

    assertEquals(
      EditorInteractionTapDispatch(position = Offset.Zero, clickCount = 1),
      gesture
        .onPointerUp(pointerId = 3L, position = Offset.Zero, nowMillis = 700L, canFinish = { true })
        .tapDispatch,
    )
  }

  @Test
  fun `second pointer cancels active tap and ignores until every pointer is up`() {
    val gesture = EditorTapGesture(tapSlopPx = 8f)

    gesture.onPointerDown(pointerId = 1L, position = Offset.Zero, canStart = { true })
    val secondDown =
      gesture.onPointerDown(pointerId = 2L, position = Offset(4f, 4f), canStart = { true })

    assertTrue(secondDown.cancelPointerStream)
    assertTrue(gesture.isIgnoringUntilAllPointersUp)
    assertNull(gesture.onTapTimer(nowMillis = 150L))

    gesture.onPointerUp(
      pointerId = 1L,
      position = Offset.Zero,
      nowMillis = 160L,
      canFinish = { true },
    )
    assertTrue(gesture.isIgnoringUntilAllPointersUp)

    gesture.onPointerUp(
      pointerId = 2L,
      position = Offset(4f, 4f),
      nowMillis = 170L,
      canFinish = { true },
    )
    assertFalse(gesture.isIgnoringUntilAllPointersUp)
  }
}
