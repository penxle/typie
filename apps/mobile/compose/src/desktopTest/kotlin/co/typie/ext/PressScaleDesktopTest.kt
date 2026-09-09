package co.typie.ext

import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.PressInteraction
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.screen.editor.editor.toolbar.emitPressInteractions
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class PressScaleDesktopTest {
  @Test
  fun `removing the toolbar input surface cancels its held press`() = runComposeUiTest {
    val source = MutableInteractionSource()
    var showInput by mutableStateOf(true)
    mainClock.autoAdvance = false
    setContent {
      CompositionLocalProvider(LocalInteractionSource provides source) {
        Box(Modifier.size(200.dp).pressScale(0.9f).testTag("surface")) {
          if (showInput) Box(Modifier.fillMaxSize().emitPressInteractions(source))
        }
      }
    }
    val restingWidth = onNodeWithTag("surface").fetchSemanticsNode().boundsInRoot.width
    onNodeWithTag("surface").performTouchInput { down(center) }
    mainClock.advanceTimeBy(160)
    assertTrue(onNodeWithTag("surface").fetchSemanticsNode().boundsInRoot.width < restingWidth)
    runOnIdle { showInput = false }
    mainClock.advanceTimeBy(160)
    assertEquals(
      restingWidth,
      onNodeWithTag("surface").fetchSemanticsNode().boundsInRoot.width,
      0.01f,
    )
  }

  @Test
  fun `rapid taps settle after the last release without replaying earlier presses`() =
    runComposeUiTest {
      val source = MutableInteractionSource()
      mainClock.autoAdvance = false
      setContent {
        CompositionLocalProvider(LocalInteractionSource provides source) {
          Box(Modifier.size(200.dp).pressScale(0.9f).testTag("surface"))
        }
      }
      val restingWidth = onNodeWithTag("surface").fetchSemanticsNode().boundsInRoot.width
      repeat(5) {
        val press = PressInteraction.Press(Offset.Zero)
        runOnIdle { source.tryEmit(press) }
        mainClock.advanceTimeBy(32)
        runOnIdle { source.tryEmit(PressInteraction.Release(press)) }
        mainClock.advanceTimeBy(32)
      }
      mainClock.advanceTimeBy(160)
      assertEquals(
        restingWidth,
        onNodeWithTag("surface").fetchSemanticsNode().boundsInRoot.width,
        0.01f,
      )
    }

  @Test
  fun `releasing one of two presses keeps the surface pressed until the other is cancelled`() =
    runComposeUiTest {
      val source = MutableInteractionSource()
      mainClock.autoAdvance = false
      setContent {
        CompositionLocalProvider(LocalInteractionSource provides source) {
          Box(Modifier.size(200.dp).pressScale(0.9f).testTag("surface"))
        }
      }
      val restingWidth = onNodeWithTag("surface").fetchSemanticsNode().boundsInRoot.width
      val first = PressInteraction.Press(Offset.Zero)
      val second = PressInteraction.Press(Offset.Zero)
      runOnIdle { source.tryEmit(first) }
      mainClock.advanceTimeBy(160)
      runOnIdle { source.tryEmit(second) }
      mainClock.advanceTimeBy(160)
      runOnIdle { source.tryEmit(PressInteraction.Release(first)) }
      mainClock.advanceTimeBy(160)
      assertTrue(onNodeWithTag("surface").fetchSemanticsNode().boundsInRoot.width < restingWidth)
      runOnIdle { source.tryEmit(PressInteraction.Cancel(second)) }
      mainClock.advanceTimeBy(160)
      assertEquals(
        restingWidth,
        onNodeWithTag("surface").fetchSemanticsNode().boundsInRoot.width,
        0.01f,
      )
    }
}
