package co.typie.ui.component.tooltip

import androidx.compose.foundation.background
import androidx.compose.foundation.focusable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asSkiaBitmap
import androidx.compose.ui.graphics.toPixelMap
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertIsFocused
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.click
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performKeyInput
import androidx.compose.ui.test.performMouseInput
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.pressKey
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.ext.clickable
import co.typie.icons.Lucide
import co.typie.platform.LocalHardwareKeyboardConnected
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.ui.component.CardRow
import co.typie.ui.component.Text
import co.typie.ui.component.popover.LocalPopoverOverlayState
import co.typie.ui.component.popover.LocalPopoverPaneRenderPhase
import co.typie.ui.component.popover.PopoverMenu
import co.typie.ui.component.popover.PopoverOverlay
import co.typie.ui.component.popover.PopoverOverlayState
import co.typie.ui.component.popover.PopoverPaneRenderPhase
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.input.hoverFeedback
import co.typie.ui.theme.DarkColors
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import org.jetbrains.skia.EncodedImageFormat
import org.jetbrains.skia.Image

@OptIn(ExperimentalTestApi::class)
class TooltipDesktopTest {

  @Test
  fun controlHoverChangesOnlyItsBackgroundAndStopsWhenDisabled() = runComposeUiTest {
    mainClock.autoAdvance = false
    val enabled = mutableStateOf(true)
    setContent {
      val source = remember { MutableInteractionSource() }
      Box(
        Modifier.size(64.dp)
          .testTag("control")
          .background(Color.White)
          .hoverFeedback(source, enabled.value, RoundedCornerShape(8.dp))
          .clickable(enabled = enabled.value, interactionSource = source) {}
      ) {
        Box(Modifier.align(Alignment.Center).size(8.dp).background(Color.Red))
      }
    }
    val control = onNodeWithTag("control")
    val resting = control.captureToImage().toPixelMap()[20, 20]
    control.performMouseInput { moveTo(center) }
    mainClock.advanceTimeBy(200)
    val hovered = control.captureToImage().toPixelMap()[20, 20]
    assertEquals(Color.Red, control.captureToImage().toPixelMap()[32, 32])
    kotlin.test.assertTrue(hovered.red < resting.red, "The control owns its hovered background")
    control.performMouseInput { press() }
    mainClock.advanceTimeBy(200)
    val pressed = control.captureToImage().toPixelMap()[20, 20]
    kotlin.test.assertTrue(
      pressed.red < hovered.red,
      "Pressed feedback must be stronger than hover",
    )
    control.performMouseInput { release() }
    runOnIdle { enabled.value = false }
    mainClock.advanceTimeBy(200)
    assertEquals(resting, control.captureToImage().toPixelMap()[20, 20])
  }

  @Test
  fun sharedRowsPreserveSelectionAndDisabledAppearance() = runComposeUiTest {
    val selected = mutableStateOf(true)
    val enabled = mutableStateOf(true)
    var clicks = 0
    setContent {
      Box(Modifier.size(100.dp).background(Color.White)) {
        CardRow(
          onClick = { clicks++ },
          selected = selected.value,
          enabled = enabled.value,
          modifier = Modifier.testTag("row"),
        ) {
          Box(Modifier.size(32.dp))
        }
      }
    }
    val row = onNodeWithTag("row")
    val resting = row.captureToImage().toPixelMap()[20, 20]
    row.performMouseInput { moveTo(center) }
    assertEquals(resting, row.captureToImage().toPixelMap()[20, 20])
    runOnIdle { selected.value = false }
    kotlin.test.assertNotEquals(resting, row.captureToImage().toPixelMap()[20, 20])
    runOnIdle { enabled.value = false }
    assertEquals(resting, row.captureToImage().toPixelMap()[20, 20])
    row.performMouseInput { click() }
    assertEquals(0, clicks)
  }

  @Test
  fun openingMenuRemovesAnchorTooltipAndLeavesMenuItemsClickable() = runComposeUiTest {
    mainClock.autoAdvance = false
    lateinit var state: TooltipState
    val popover = PopoverOverlayState()
    var selected = 0
    setContent {
      val scope = rememberCoroutineScope()
      state = remember { TooltipState(scope) }
      CompositionLocalProvider(
        LocalTooltipState provides state,
        LocalPopoverOverlayState provides popover,
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        Box(Modifier.size(320.dp)) {
          Box(Modifier.padding(start = 20.dp, top = 80.dp)) {
            PopoverMenu(
              anchor = {
                TopBarButton(Lucide.Ellipsis, "메뉴", modifier = Modifier.testTag("anchor"))
              }
            ) {
              item(
                content = {
                  val tag =
                    if (LocalPopoverPaneRenderPhase.current == PopoverPaneRenderPhase.Interactive) {
                      Modifier.testTag("item")
                    } else Modifier
                  Box(tag.size(100.dp, 42.dp)) { Text("메뉴 항목") }
                }
              ) {
                selected++
              }
            }
          }
          PopoverOverlay(popover)
          TooltipHost(state)
        }
      }
    }
    onNodeWithTag("anchor").performMouseInput { moveTo(center) }
    mainClock.advanceTimeBy(1000)
    runOnIdle { assertEquals("메뉴", state.active?.text) }
    onNodeWithTag("anchor").performMouseInput { click() }
    mainClock.advanceTimeBy(1000)
    runOnIdle { assertNull(state.active) }
    onNodeWithTag("item", useUnmergedTree = true).performMouseInput { click() }
    mainClock.advanceTimeBy(500)
    runOnIdle { assertEquals(1, selected) }
  }

  @Test
  fun tooltipPreservesFocusAndUpdatesThemeAndConnectedKeyboardShortcut() = runComposeUiTest {
    mainClock.autoAdvance = false
    val keyboardConnected = mutableStateOf(false)
    val colors = mutableStateOf(LightColors)
    val editorFocus = FocusRequester()
    lateinit var state: TooltipState
    setContent {
      val scope = rememberCoroutineScope()
      state = remember { TooltipState(scope) }
      CompositionLocalProvider(
        LocalTooltipState provides state,
        LocalHardwareKeyboardConnected provides keyboardConnected.value,
        LocalAppColors provides colors.value,
        LocalThemeMode provides
          if (colors.value == DarkColors) ResolvedThemeMode.Dark else ResolvedThemeMode.Light,
      ) {
        Box(
          Modifier.size(320.dp, 320.dp).testTag("preview").background(colors.value.surfaceDefault)
        ) {
          Box(Modifier.size(1.dp).testTag("editor").focusRequester(editorFocus).focusable())
          Row(Modifier.align(Alignment.Center)) {
            EditorToolbarButton(
              Lucide.Bold,
              "굵게",
              onClick = {},
              shortcut = "⌘B",
              modifier = Modifier.testTag("bold"),
            )
            EditorToolbarButton(Lucide.Italic, "기울임", onClick = {}, shortcut = "⌘I")
            EditorToolbarButton(Lucide.Underline, "밑줄", onClick = {}, shortcut = "⌘U")
          }
          TooltipHost(state)
        }
      }
    }
    runOnIdle { editorFocus.requestFocus() }
    onNodeWithTag("bold").performMouseInput { moveTo(center) }
    mainClock.advanceTimeBy(1000)
    runOnIdle {
      assertEquals("굵게", state.active?.text)
      assertNull(state.active?.shortcut)
    }
    onNodeWithTag("editor").assertIsFocused()
    runOnIdle { keyboardConnected.value = true }
    mainClock.advanceTimeBy(500)
    runOnIdle { assertEquals("⌘B", state.active?.shortcut) }
    for ((name, theme, background) in
      listOf(
        Triple("light", LightColors, LightColors.surfaceInverse),
        Triple("dark", DarkColors, DarkColors.surfaceActive),
      )) {
      runOnIdle { colors.value = theme }
      mainClock.advanceTimeBy(500)
      val image = onNodeWithTag("preview").captureToImage()
      val pixels = image.toPixelMap()
      kotlin.test.assertTrue(
        (0 until pixels.width).any { x ->
          (0 until pixels.height).any { y -> pixels[x, y] == background }
        },
        "Tooltip surface should use the current app theme",
      )
      val directory = File("build/reports/tooltip").apply { mkdirs() }
      File(directory, "$name.png")
        .writeBytes(
          Image.makeFromBitmap(image.asSkiaBitmap()).encodeToData(EncodedImageFormat.PNG)!!.bytes
        )
    }
    runOnIdle { keyboardConnected.value = false }
    mainClock.advanceTimeByFrame()
    runOnIdle { assertNull(state.active?.shortcut) }
    onNodeWithTag("editor").performKeyInput { pressKey(Key.Escape) }
    mainClock.advanceTimeBy(200)
    runOnIdle { assertEquals("굵게", state.active?.text) }
    onNodeWithTag("editor").assertIsFocused()
  }

  @Test
  fun hoverTransfersAndClickPassesThroughWithoutReopeningTooltip() = runComposeUiTest {
    mainClock.autoAdvance = false
    var clicks = 0
    lateinit var state: TooltipState
    setContent {
      val scope = rememberCoroutineScope()
      state = remember { TooltipState(scope) }
      CompositionLocalProvider(
        LocalTooltipState provides state,
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        Box(Modifier.size(320.dp).background(LightColors.surfaceDefault)) {
          Row(Modifier.align(Alignment.Center)) {
            listOf("첫째", "둘째").forEach { label ->
              Box(Modifier.size(64.dp).testTag(label).tooltip(label).clickable { clicks++ }) {
                Text(label)
              }
            }
          }
          TooltipHost(state)
        }
      }
    }
    onNodeWithTag("첫째").performMouseInput { moveTo(center) }
    mainClock.advanceTimeBy(400)
    runOnIdle { assertNull(state.active) }
    mainClock.advanceTimeBy(200)
    runOnIdle { assertEquals("첫째", state.active?.text) }
    onNodeWithTag("둘째").performMouseInput { moveTo(center) }
    mainClock.advanceTimeByFrame()
    runOnIdle { assertEquals("둘째", state.active?.text) }
    onNodeWithTag("둘째").performMouseInput { click() }
    mainClock.advanceTimeBy(1000)
    runOnIdle {
      assertEquals(1, clicks)
      assertNull(state.active)
    }
  }

  @Test
  fun touchDoesNotOpenTooltipAndDisablingCancelsPendingHover() = runComposeUiTest {
    mainClock.autoAdvance = false
    val enabled = mutableStateOf(true)
    lateinit var state: TooltipState
    var clicks = 0
    setContent {
      val scope = rememberCoroutineScope()
      state = remember { TooltipState(scope) }
      CompositionLocalProvider(
        LocalTooltipState provides state,
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        Box(Modifier.fillMaxSize().padding(40.dp)) {
          Box(
            Modifier.size(64.dp).testTag("button").tooltip("설명", enabled = enabled.value).clickable(
              enabled = enabled.value
            ) {
              clicks++
            }
          )
          TooltipHost(state)
        }
      }
    }
    onNodeWithTag("button").performTouchInput { click() }
    mainClock.advanceTimeBy(1000)
    runOnIdle {
      assertEquals(1, clicks)
      assertNull(state.active)
    }
    onNodeWithTag("button").performMouseInput { moveTo(center) }
    mainClock.advanceTimeBy(200)
    runOnIdle { enabled.value = false }
    mainClock.advanceTimeBy(1000)
    runOnIdle { assertNull(state.active) }
  }
}
