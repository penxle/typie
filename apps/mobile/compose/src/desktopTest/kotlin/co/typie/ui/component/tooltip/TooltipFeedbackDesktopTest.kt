package co.typie.ui.component.tooltip

import androidx.compose.foundation.background
import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PixelMap
import androidx.compose.ui.graphics.graphicsLayer
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
import co.typie.ui.input.PointerInputModeState
import co.typie.ui.input.trackPointerInputMode
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class TooltipFeedbackDesktopTest {
  @Test
  fun tooltipUsesWebLineBoxesRatherThanTrimmedFontMetrics() = runComposeUiTest {
    lateinit var withoutShortcut: TooltipContent
    lateinit var withShortcut: TooltipContent
    setContent {
      withoutShortcut = rememberTooltipContent("굵게", null, 280)
      withShortcut = rememberTooltipContent("굵게", "⌘B", 280)
    }
    runOnIdle {
      assertEquals(17, withoutShortcut.size.height, "The body must keep its 1.4em line box")
      assertEquals(
        12,
        withShortcut.size.height - withoutShortcut.size.height,
        "The shortcut must add a 1em line box without an extra gap",
      )
    }
  }

  @Test
  fun transferredTooltipTracksMovingAnchorWithoutAnotherTravelAnimation() = runComposeUiTest {
    mainClock.autoAdvance = false
    val translation = mutableStateOf(0f)
    setContent {
      val scope = rememberCoroutineScope()
      val state = remember { TooltipState(scope) }
      CompositionLocalProvider(
        LocalTooltipState provides state,
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        Box(Modifier.size(320.dp).background(Color.White).testTag("root")) {
          Row(Modifier.align(Alignment.Center)) {
            Box(Modifier.size(48.dp).tooltip("처음").testTag("first"))
            Box(
              Modifier.graphicsLayer { translationY = translation.value }
                .size(48.dp)
                .tooltip("다음")
                .testTag("second")
            )
          }
          TooltipHost(state)
        }
      }
    }
    onNodeWithTag("first").performMouseInput { moveTo(center) }
    mainClock.advanceTimeBy(1000)
    onNodeWithTag("second").performMouseInput { moveTo(center) }
    mainClock.advanceTimeBy(300)
    val before = onNodeWithTag("root").captureToImage().toPixelMap().opaqueTop()
    runOnIdle { translation.value = 20f }
    mainClock.advanceTimeBy(32)
    val after = onNodeWithTag("root").captureToImage().toPixelMap().opaqueTop()
    assertEquals(20, after - before, "Scrolling must immediately track the anchor after a transfer")
  }

  @Test
  fun fullyClippedAnchorClosesTooltipWithoutPointerExit() = runComposeUiTest {
    mainClock.autoAdvance = false
    val translation = mutableStateOf(0f)
    lateinit var state: TooltipState
    setContent {
      val scope = rememberCoroutineScope()
      state = remember { TooltipState(scope) }
      CompositionLocalProvider(
        LocalTooltipState provides state,
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        Box(Modifier.size(320.dp)) {
          Box(Modifier.align(Alignment.Center).size(80.dp).clipToBounds()) {
            Box(
              Modifier.graphicsLayer { translationY = translation.value }
                .size(80.dp)
                .tooltip("스크롤")
                .testTag("anchor")
            )
          }
          TooltipHost(state)
        }
      }
    }
    onNodeWithTag("anchor").performMouseInput { moveTo(center) }
    mainClock.advanceTimeBy(1000)
    assertNotNull(state.active)
    runOnIdle { translation.value = 30f }
    mainClock.advanceTimeBy(64)
    assertNotNull(state.active, "Partially visible anchors still own the tooltip")
    runOnIdle { translation.value = 100f }
    mainClock.advanceTimeBy(64)
    assertNull(state.active, "A retained but fully clipped anchor must close its tooltip")
  }

  @Test
  fun keyboardFocusAloneDoesNotOpenHoverTooltip() = runComposeUiTest {
    mainClock.autoAdvance = false
    val focus = FocusRequester()
    lateinit var state: TooltipState
    setContent {
      val scope = rememberCoroutineScope()
      state = remember { TooltipState(scope) }
      CompositionLocalProvider(
        LocalTooltipState provides state,
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        Box(Modifier.size(80.dp).tooltip("포커스").focusRequester(focus).focusable())
        TooltipHost(state)
      }
    }
    runOnIdle { focus.requestFocus() }
    mainClock.advanceTimeBy(1000)
    assertNull(state.active)
  }

  @Test
  fun tooltipArrowFadesWithSurfaceAndShrinkStartsWithContentChange() = runComposeUiTest {
    mainClock.autoAdvance = false
    setContent {
      val state = rememberCoroutineScope().let { scope -> remember { TooltipState(scope) } }
      CompositionLocalProvider(
        LocalTooltipState provides state,
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        Box(Modifier.size(320.dp).background(Color.White).testTag("root")) {
          Row(Modifier.align(Alignment.Center)) {
            Box(
              Modifier.size(48.dp)
                .tooltip("아주 긴 툴팁 콘텐츠", placement = TooltipPlacement.Above)
                .testTag("long")
            )
            Box(
              Modifier.size(48.dp)
                .tooltip("짧음", placement = TooltipPlacement.Above)
                .testTag("short")
            )
          }
          TooltipHost(state)
        }
      }
    }
    onNodeWithTag("long").performMouseInput { moveTo(center) }
    mainClock.advanceTimeBy(600)
    val introPixels = onNodeWithTag("root").captureToImage().toPixelMap()
    val intro = introPixels.inkRows()
    val bodyRow =
      (0 until introPixels.height).maxBy { y ->
        (0 until introPixels.width).count { x -> introPixels[x, y].red < 0.5f }
      }
    val bodyLeft = (0 until introPixels.width).first { x -> introPixels[x, bodyRow].red < 0.5f }
    assertTrue(
      introPixels[bodyLeft - 3, bodyRow].red < 0.999f,
      "The shadow must extend outside the body during intro, before opacity reaches one",
    )
    assertTrue(intro.isNotEmpty())
    assertTrue(intro.last() < intro.max() / 2, "Arrow must already be drawn during intro: $intro")
    mainClock.advanceTimeBy(200)
    val oldWidth = onNodeWithTag("root").captureToImage().toPixelMap().inkRows().max()
    onNodeWithTag("short").performMouseInput { moveTo(center) }
    mainClock.advanceTimeBy(64)
    val midway = onNodeWithTag("root").captureToImage().toPixelMap().inkRows().max()
    mainClock.advanceTimeBy(300)
    val newWidth = onNodeWithTag("root").captureToImage().toPixelMap().inkRows().max()
    assertTrue(
      newWidth < midway && midway < oldWidth,
      "Size must animate during crossfade: $oldWidth -> $midway -> $newWidth",
    )
  }

  @Test
  fun wheelAndLayerMovementKeepTooltipAtTheLiveAnchor() = runComposeUiTest {
    mainClock.autoAdvance = false
    val translation = mutableStateOf(0f)
    lateinit var state: TooltipState
    setContent {
      val scope = rememberCoroutineScope()
      state = remember { TooltipState(scope) }
      CompositionLocalProvider(
        LocalTooltipState provides state,
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        Box(Modifier.size(320.dp).background(Color.White).testTag("root")) {
          Box(Modifier.align(Alignment.Center).graphicsLayer { translationY = translation.value }) {
            Box(Modifier.size(80.dp).testTag("anchor").tooltip("스크롤 유지"))
          }
          TooltipHost(state)
        }
      }
    }
    onNodeWithTag("anchor").performMouseInput { moveTo(center) }
    mainClock.advanceTimeBy(1000)
    val anchor = assertNotNull(state.active)
    val initialTop = anchor.bounds.top
    onNodeWithTag("anchor").performMouseInput { scroll(10f) }
    mainClock.advanceTimeBy(200)
    assertEquals(anchor, state.active)
    runOnIdle { translation.value = 12f }
    mainClock.advanceTimeBy(200)
    assertEquals(anchor, state.active)
    assertEquals(initialTop + 12f, anchor.bounds.top, 0.1f)
  }

  @Test
  fun leavingEditorForTopBarPreservesPointerModeUntilTouchInput() = runComposeUiTest {
    val mode = PointerInputModeState()
    setContent {
      Box(Modifier.size(320.dp)) {
        Box(Modifier.size(320.dp, 64.dp).testTag("topbar").clickable {})
        Box(
          Modifier.offset(y = 64.dp)
            .size(320.dp, 256.dp)
            .trackPointerInputMode(mode)
            .testTag("editor")
        )
      }
    }
    onNodeWithTag("editor").performMouseInput { moveTo(center) }
    assertTrue(mode.nonTouchPointerActive)
    onNodeWithTag("topbar").performMouseInput { moveTo(center) }
    assertTrue(mode.nonTouchPointerActive)
    onNodeWithTag("editor").performTouchInput { click() }
    assertFalse(mode.nonTouchPointerActive)
  }
}

private fun PixelMap.inkRows(): List<Int> =
  (0 until height)
    .map { y -> (0 until width).count { x -> this[x, y].red < 0.95f } }
    .filter { it > 0 }

private fun PixelMap.opaqueTop(): Int =
  (0 until height).first { y -> (0 until width).count { x -> this[x, y].red < 0.3f } > 12 }
