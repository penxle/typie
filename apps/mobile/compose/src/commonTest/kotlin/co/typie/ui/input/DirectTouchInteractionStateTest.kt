package co.typie.ui.input

import androidx.compose.ui.input.pointer.PointerType
import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class DirectTouchInteractionStateTest {
  @Test
  fun `pointer and keyboard interactions update the app input state`() {
    val state = DirectTouchInteractionState(initialDirectTouchInteraction = false)

    state.recordPointerInteraction(PointerType.Touch)
    assertTrue(state.isDirectTouchInteraction)

    state.recordPointerInteraction(PointerType.Stylus)
    assertTrue(state.isDirectTouchInteraction)

    state.recordPointerInteraction(PointerType.Mouse)
    assertFalse(state.isDirectTouchInteraction)

    state.recordPointerInteraction(PointerType.Touch)
    state.recordKeyboardInteraction()
    assertFalse(state.isDirectTouchInteraction)
  }
}
