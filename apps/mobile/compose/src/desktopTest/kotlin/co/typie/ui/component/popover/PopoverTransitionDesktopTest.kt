package co.typie.ui.component.popover

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toPixelMap
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.click
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performMouseInput
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import co.typie.icons.Lucide
import co.typie.ui.component.sheet.SheetBarButton
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class PopoverTransitionDesktopTest {
  @Test fun mousePressScalesTheTopBarTrigger() = verifyPressedTrigger(mouse = true)

  @Test fun touchPressScalesTheTopBarTrigger() = verifyPressedTrigger(mouse = false)

  @Test fun mousePressScalesTheSheetBarTrigger() = verifyPressedTrigger(mouse = true, sheet = true)

  @Test fun touchPressScalesTheSheetBarTrigger() = verifyPressedTrigger(mouse = false, sheet = true)

  private fun verifyPressedTrigger(mouse: Boolean, sheet: Boolean = false) = runComposeUiTest {
    mainClock.autoAdvance = false
    val state = PopoverOverlayState()
    setContent {
      CompositionLocalProvider(
        LocalPopoverOverlayState provides state,
        LocalThemeMode provides ResolvedThemeMode.Light,
        LocalDensity provides Density(1f),
      ) {
        Box(Modifier.size(400.dp, 360.dp).background(Color.White).testTag("root")) {
          Box(Modifier.align(Alignment.TopEnd).padding(20.dp)) {
            PopoverMenu(
              anchor = {
                if (sheet) {
                  SheetBarButton(
                    Lucide.Ellipsis,
                    "도구",
                    backgroundColor = Color.Red,
                    modifier = Modifier.testTag("anchor"),
                  )
                } else {
                  TopBarButton(
                    Lucide.Ellipsis,
                    "도구",
                    backgroundColor = Color.Red,
                    modifier = Modifier.testTag("anchor"),
                  )
                }
              }
            ) {
              item(Lucide.Check, "항목") {}
            }
          }
          PopoverOverlay(state)
        }
      }
    }
    val root = onNodeWithTag("root")
    val anchor = onNodeWithTag("anchor").fetchSemanticsNode().boundsInRoot
    val edgeX = anchor.left.toInt() - 1
    val edgeY = anchor.center.y.toInt()
    val idleColor = root.captureToImage().toPixelMap()[edgeX, edgeY]
    if (mouse)
      root.performMouseInput {
        moveTo(anchor.center)
        press()
      }
    else root.performTouchInput { down(anchor.center) }
    mainClock.advanceTimeBy(128)
    waitForIdle()
    assertEquals(null, state.entry, "Press feedback must appear before the hold opens the menu")
    val pressedColor = root.captureToImage().toPixelMap()[edgeX, edgeY]
    assertTrue(
      pressedColor.green < idleColor.green - 0.05f,
      "Pressing must expand the button beyond its resting bounds",
    )
    mainClock.advanceTimeBy(80)
    waitForIdle()
    val scale = checkNotNull(state.entry?.anchorSurface?.scale)
    assertEquals(1.1f, scale.value, 0.001f, "The popover must inherit the pressed button's scale")
    if (mouse) root.performMouseInput { release() } else root.performTouchInput { up() }
    mainClock.advanceTimeBy(600)
    waitForIdle()
    assertEquals(1f, scale.value)
    assertTrue(state.acceptsInput)
  }

  @Test fun roundTriggerKeepsContentInsideTheMovingSurface() = verifySurface(wide = false)

  @Test fun widerTriggerCanShrinkIntoANarrowerMenu() = verifySurface(wide = true)

  @Test
  fun bareTriggerRevealsTheMenuWithoutADarkFlash() = verifySurface(wide = false, bordered = false)

  private fun verifySurface(wide: Boolean, bordered: Boolean = true) = runComposeUiTest {
    mainClock.autoAdvance = false
    val state = PopoverOverlayState()
    setContent {
      CompositionLocalProvider(
        LocalPopoverOverlayState provides state,
        LocalThemeMode provides ResolvedThemeMode.Light,
        LocalDensity provides Density(1f),
      ) {
        Box(Modifier.size(400.dp, 360.dp).background(Color.White).testTag("root")) {
          Box(Modifier.align(Alignment.TopEnd).padding(20.dp)) {
            PopoverMenu(
              minWidth = 220.dp,
              maxWidth = 220.dp,
              anchor = {
                Box(
                  Modifier.size(if (wide) 300.dp else 44.dp, 44.dp)
                    .then(
                      if (bordered)
                        Modifier.popoverAnchorSurface(
                          Color.White,
                          Color.Red,
                          if (wide) 12.dp else null,
                        )
                      else Modifier
                    )
                    .testTag("anchor")
                )
              },
            ) {
              repeat(4) {
                item(content = { Box(Modifier.size(208.dp, 42.dp).background(Color.Green)) }) {}
              }
            }
          }
          PopoverOverlay(state)
        }
      }
    }
    onNodeWithTag("anchor").performMouseInput { click() }
    mainClock.advanceTimeByFrame()
    waitForIdle()
    val anchor = state.anchorBounds
    var checkedFrames = 0
    repeat(12) {
      mainClock.advanceTimeByFrame()
      waitForIdle()
      val progress = state.progress
      if (progress <= 0f || progress >= 1f) return@repeat
      val pane = checkNotNull(state.paneBoundsInWindow)
      val left = anchor.left + (pane.left - anchor.left) * progress
      val top = anchor.top + (pane.top - anchor.top) * progress
      val right = anchor.right + (pane.right - anchor.right) * progress
      val bottom = anchor.bottom + (pane.bottom - anchor.bottom) * progress
      val pixels = onNodeWithTag("root").captureToImage().toPixelMap()
      var greenPixels = 0
      for (y in 0 until pixels.height) {
        for (x in 0 until pixels.width) {
          val color = pixels[x, y]
          if (color.green > color.red + 0.04f && color.green > color.blue + 0.04f) {
            greenPixels++
            assertTrue(
              x >= left - 1 && x <= right + 1 && y >= top - 1 && y <= bottom + 1,
              "Menu content escaped the surface at progress=$progress, pixel=($x, $y)",
            )
          }
        }
      }
      if (progress > 0.4f) assertTrue(greenPixels > 0, "The expanding menu must reveal its content")
      // The original red trigger outline must not remain inside the expanded surface.
      if (progress > 0.4f && !wide) {
        val color = pixels[anchor.left + 1, anchor.top + anchor.height / 2]
        assertFalse(color.red > color.green + 0.1f && color.red > color.blue + 0.1f)
      }
      if (wide && progress < 0.5f) {
        // The outer menu's final width must not constrain the still-wider trigger surface.
        val color = pixels[(right - 1).toInt(), (top + (bottom - top) / 2).toInt()]
        assertTrue(
          color.red > color.blue + 0.02f,
          "The right edge must stay attached to the trigger",
        )
      }
      if (!bordered && progress > 0.2f && progress < 0.8f) {
        val paddingColor = pixels[(right - 3).toInt(), (top + (bottom - top) / 2).toInt()]
        assertTrue(
          paddingColor.red > 0.92f,
          "A transparent trigger must not tint the light menu black",
        )
      }
      checkedFrames++
    }
    assertTrue(checkedFrames >= 3)
  }

  @Test
  fun closingDuringIntroContinuesFromTheVisibleSize() = runComposeUiTest {
    mainClock.autoAdvance = false
    val state = PopoverOverlayState()
    setContent {
      CompositionLocalProvider(
        LocalPopoverOverlayState provides state,
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        Box(Modifier.size(400.dp)) {
          PopoverMenu(anchor = { Box(Modifier.size(44.dp).testTag("anchor")) }) {
            item(content = { Box(Modifier.size(180.dp, 42.dp)) }) {}
          }
          PopoverOverlay(state)
        }
      }
    }
    onNodeWithTag("anchor").performMouseInput { click() }
    mainClock.advanceTimeBy(48)
    waitForIdle()
    val interruptedProgress = state.progress
    assertTrue(interruptedProgress > 0f && interruptedProgress < 1f)
    runOnIdle { state.dismissFromOutsideGesture() }
    var previous = interruptedProgress
    repeat(15) {
      mainClock.advanceTimeByFrame()
      waitForIdle()
      assertTrue(state.progress <= previous, "Closing must never jump to a larger surface")
      previous = state.progress
    }
    assertEquals(0f, state.progress)
    assertEquals(null, state.entry)
  }
}
