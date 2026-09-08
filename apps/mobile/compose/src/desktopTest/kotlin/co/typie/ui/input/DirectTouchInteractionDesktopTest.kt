package co.typie.ui.input

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.InternalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.SkikoComposeUiTest
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class, ExperimentalComposeUiApi::class, InternalComposeUiApi::class)
class DirectTouchInteractionDesktopTest {
  @Test
  fun indirectPointerInputReplacesTouchWithoutAPress() = runComposeUiTest {
    val state = DirectTouchInteractionState(initialDirectTouchInteraction = false)
    setContent { Box(Modifier.size(100.dp).testTag("input").trackDirectTouchInteraction(state)) }
    val scene = (this as SkikoComposeUiTest).scene
    for (eventType in
      listOf(PointerEventType.Scroll, PointerEventType.PanStart, PointerEventType.ScaleStart)) {
      onNodeWithTag("input").performTouchInput {
        down(center)
        up()
      }
      runOnIdle { assertTrue(state.isDirectTouchInteraction) }
      runOnUiThread {
        scene.sendPointerEvent(
          eventType = eventType,
          position = Offset(50f, 50f),
          scrollDelta = Offset(0f, 10f),
        )
      }
      runOnIdle { assertFalse(state.isDirectTouchInteraction) }
    }
  }
}
