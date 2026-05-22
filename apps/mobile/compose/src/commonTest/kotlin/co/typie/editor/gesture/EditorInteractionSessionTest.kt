package co.typie.editor.gesture

import androidx.compose.ui.geometry.Offset
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull

class EditorInteractionSessionTest {
  @Test
  fun `pinch start cancels active pending tap stream`() {
    val session = EditorInteractionSession(tapSlopPx = 8f)

    session.onPointerDown(pointerId = 1L, position = Offset(10f, 20f), nowMillis = 0L)

    assertEquals(
      EditorInteractionPointerResult(cancelTapDispatch = true, cancelPointerStream = true),
      session.applyEvent(EditorInteractionEvent.PinchStart),
    )
    assertEquals(EditorInteractionMode.Pinching, session.interactionMode)
    assertFalse(session.hasActivePointer)
    assertNull(session.onTapTimer(nowMillis = 150L))
  }
}
