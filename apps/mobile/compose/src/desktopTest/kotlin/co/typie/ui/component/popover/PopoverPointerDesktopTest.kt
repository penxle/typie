package co.typie.ui.component.popover

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.toPixelMap
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.click
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performMouseInput
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.ext.clickable
import co.typie.icons.Lucide
import co.typie.ui.component.sheet.SheetBarButton
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class PopoverPointerDesktopTest {
  @Test
  fun outsideMouseAndTouchDismissWithoutActivatingUnderlyingControl() = runComposeUiTest {
    val state = PopoverOverlayState()
    var clicks = 0
    setContent {
      CompositionLocalProvider(
        LocalPopoverOverlayState provides state,
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        Box(Modifier.size(320.dp).popoverOutsideTapHost(state)) {
          Box(
            Modifier.align(Alignment.BottomCenter).size(100.dp).testTag("outside").clickable {
              clicks++
            }
          )
          PopoverMenu(
            anchor = {
              SheetBarButton(Lucide.ListFilter, "필터", modifier = Modifier.testTag("anchor"))
            }
          ) {
            item(Lucide.Check, "항목") {}
          }
          PopoverOverlay(state)
        }
      }
    }
    repeat(2) { index ->
      onNodeWithTag("anchor").performMouseInput { click() }
      waitForIdle()
      assertTrue(state.acceptsInput)
      if (index == 0) onNodeWithTag("outside").performMouseInput { click() }
      else onNodeWithTag("outside").performTouchInput { click() }
      waitForIdle()
      assertFalse(state.acceptsInput)
      assertEquals(0, clicks)
    }
    onNodeWithTag("outside").performMouseInput { click() }
    waitForIdle()
    assertEquals(1, clicks)
  }

  @Test
  fun transparentAnchorCannotBlockHoverOrClickOnTheMenuTrailingEdge() = runComposeUiTest {
    mainClock.autoAdvance = false
    val state = PopoverOverlayState()
    var selections = 0
    setContent {
      CompositionLocalProvider(
        LocalPopoverOverlayState provides state,
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        Box(Modifier.size(320.dp).testTag("root")) {
          Box(Modifier.align(Alignment.TopEnd)) {
            PopoverMenu(
              anchor = {
                SheetBarButton(Lucide.ListFilter, "필터", modifier = Modifier.testTag("anchor"))
              }
            ) {
              item(
                content = {
                  val tag =
                    if (LocalPopoverPaneRenderPhase.current == PopoverPaneRenderPhase.Interactive)
                      Modifier.testTag("item")
                    else Modifier
                  Box(tag.size(180.dp, 42.dp))
                }
              ) {
                selections++
              }
            }
          }
          PopoverOverlay(state)
        }
      }
    }
    repeat(2) { index ->
      onNodeWithTag("anchor").performMouseInput { click() }
      mainClock.advanceTimeBy(600)
      val item = onNodeWithTag("item", useUnmergedTree = true)
      onNodeWithTag("root").performMouseInput { moveTo(Offset(40f, 250f)) }
      mainClock.advanceTimeBy(200)
      val resting = item.captureToImage().toPixelMap()[150, 20]
      item.performMouseInput { moveTo(Offset(width - 20f, center.y)) }
      mainClock.advanceTimeBy(200)
      val hovered = item.captureToImage().toPixelMap()[150, 20]
      assertTrue(hovered.red < resting.red, "Trailing area must receive hover")
      if (index == 0) item.performMouseInput { click() }
      else item.performTouchInput { click(Offset(width - 20f, center.y)) }
      mainClock.advanceTimeBy(600)
      assertEquals(index + 1, selections)
    }
  }
}
