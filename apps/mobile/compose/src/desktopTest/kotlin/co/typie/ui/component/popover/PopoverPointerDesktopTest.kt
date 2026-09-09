package co.typie.ui.component.popover

import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
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
import androidx.compose.ui.test.swipe
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.ext.LocalScrollGestureLockState
import co.typie.ext.ScrollGestureLockState
import co.typie.ext.clickable
import co.typie.ext.verticalScroll
import co.typie.icons.Lucide
import co.typie.ui.component.sheet.SheetBarButton
import co.typie.ui.component.tooltip.LocalTooltipState
import co.typie.ui.component.tooltip.TooltipState
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class PopoverPointerDesktopTest {
  @Test
  fun touchCanSelectAnItemWithoutReleasingTheAnchorPress() =
    selectItemFromAnchorPress(mouse = false)

  @Test
  fun mouseCanSelectAnItemWithoutReleasingTheAnchorPress() = selectItemFromAnchorPress(mouse = true)

  @Test
  fun touchCanLeaveThePaneAndReleaseWithoutSelecting() =
    selectItemFromAnchorPress(mouse = false, releaseOutside = true)

  @Test
  fun mouseCanLeaveThePaneAndReleaseWithoutSelecting() =
    selectItemFromAnchorPress(mouse = true, releaseOutside = true)

  private fun selectItemFromAnchorPress(mouse: Boolean, releaseOutside: Boolean = false) =
    runComposeUiTest {
      mainClock.autoAdvance = false
      val state = PopoverOverlayState()
      val scrollLock = ScrollGestureLockState()
      var selectedItem: Int? = null
      setContent {
        val scope = rememberCoroutineScope()
        val tooltipState = remember(scope) { TooltipState(scope) }
        CompositionLocalProvider(
          LocalPopoverOverlayState provides state,
          LocalScrollGestureLockState provides scrollLock,
          LocalTooltipState provides tooltipState,
          LocalThemeMode provides ResolvedThemeMode.Light,
        ) {
          Box(Modifier.size(320.dp).testTag("root").popoverOutsideTapHost(state)) {
            Box(Modifier.align(Alignment.TopEnd)) {
              PopoverMenu(
                anchor = {
                  SheetBarButton(Lucide.ListFilter, "필터", modifier = Modifier.testTag("anchor"))
                }
              ) {
                repeat(3) { index ->
                  item(
                    content = {
                      val tag =
                        if (
                          LocalPopoverPaneRenderPhase.current == PopoverPaneRenderPhase.Interactive
                        )
                          Modifier.testTag("item-$index")
                        else Modifier
                      Box(tag.size(180.dp, 42.dp))
                    }
                  ) {
                    selectedItem = index
                  }
                }
              }
            }
            PopoverOverlay(state)
          }
        }
      }
      val root = onNodeWithTag("root")
      val anchorPosition = onNodeWithTag("anchor").fetchSemanticsNode().boundsInRoot.center
      if (mouse)
        root.performMouseInput {
          moveTo(anchorPosition)
          press()
        }
      else root.performTouchInput { down(anchorPosition) }
      mainClock.advanceTimeBy(700)
      waitForIdle()
      assertTrue(state.acceptsInput, "Holding the anchor must open the popover")
      assertTrue(scrollLock.isLocked, "The original press must keep ownership while selecting")

      for (index in 1..2) {
        val itemPosition =
          onNodeWithTag("item-$index", useUnmergedTree = true)
            .fetchSemanticsNode()
            .boundsInRoot
            .center
        if (mouse) root.performMouseInput { moveTo(itemPosition) }
        else root.performTouchInput { moveTo(itemPosition) }
        mainClock.advanceTimeByFrame()
        waitForIdle()
        assertEquals(null, selectedItem, "Moving over an item must not activate it before release")
      }
      if (releaseOutside) {
        if (mouse) root.performMouseInput { moveTo(Offset(20f, 280f)) }
        else root.performTouchInput { moveTo(Offset(20f, 280f)) }
        mainClock.advanceTimeByFrame()
      }
      if (mouse) root.performMouseInput { release() } else root.performTouchInput { up() }
      mainClock.advanceTimeBy(600)
      waitForIdle()
      assertFalse(scrollLock.isLocked, "Release must return scrolling to the parent")
      if (releaseOutside) {
        assertEquals(null, selectedItem, "Releasing outside must cancel selection")
        assertTrue(state.acceptsInput, "The opening gesture must not count as an outside tap")
        val item = onNodeWithTag("item-2", useUnmergedTree = true)
        if (mouse) item.performMouseInput { click() } else item.performTouchInput { click() }
        mainClock.advanceTimeBy(600)
        waitForIdle()
      }
      assertEquals(2, selectedItem, "Only the final item must be selected")
      assertFalse(state.acceptsInput)
    }

  @Test
  fun draggingTheAnchorBeforeTheHoldDelayScrollsWithoutOpening() = runComposeUiTest {
    val state = PopoverOverlayState()
    val scrollLock = ScrollGestureLockState()
    val scrollState = ScrollState(0)
    setContent {
      CompositionLocalProvider(
        LocalPopoverOverlayState provides state,
        LocalScrollGestureLockState provides scrollLock,
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        Box(Modifier.size(320.dp).testTag("root").popoverOutsideTapHost(state)) {
          Column(Modifier.verticalScroll(scrollState)) {
            Box(Modifier.height(150.dp))
            PopoverMenu(
              anchor = {
                SheetBarButton(Lucide.ListFilter, "필터", modifier = Modifier.testTag("anchor"))
              }
            ) {
              item(Lucide.Check, "항목") {}
            }
            Box(Modifier.size(320.dp, 600.dp))
          }
          PopoverOverlay(state)
        }
      }
    }
    val anchorPosition = onNodeWithTag("anchor").fetchSemanticsNode().boundsInRoot.center
    onNodeWithTag("root").performTouchInput {
      swipe(anchorPosition, anchorPosition - Offset(0f, 120f), durationMillis = 100)
    }
    waitForIdle()
    assertTrue(scrollState.value > 0, "A drag before the hold must remain a scroll gesture")
    assertFalse(state.acceptsInput)
    assertFalse(scrollLock.isLocked)
  }

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
