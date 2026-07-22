package co.typie.ui.component.sheet

import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertIsFocused
import androidx.compose.ui.test.assertIsNotFocused
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.ext.clickable
import co.typie.ui.theme.LightAppShadows
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalAppShadows
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import dev.chrisbanes.haze.blur.HazeBlurStyle
import dev.chrisbanes.haze.blur.LocalHazeBlurStyle
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class SheetOverlayDesktopTest {
  @Test
  fun dismissalReleasesFocusBeforeRemoval() = runComposeUiTest {
    val sheet = Sheet()

    setContent {
      SheetTestTheme {
        Box(Modifier.size(width = 400.dp, height = 800.dp)) {
          LaunchedEffect(Unit) { sheet.present<Unit> { ClosingSheetContent() } }

          SheetOverlay(sheet)
        }
      }
    }

    waitUntil(timeoutMillis = 5_000) { sheet.entries.isNotEmpty() }
    waitForIdle()
    onNodeWithTag(SheetFocusTag).assertIsFocused()

    mainClock.autoAdvance = false
    onNodeWithTag(DismissTag).performTouchInput {
      down(center)
      up()
    }
    waitForIdle()

    assertNotNull(sheet.entries.singleOrNull())
    assertFalse(sheet.acceptsInput)
    onNodeWithTag(SheetFocusTag).assertIsNotFocused()

    mainClock.advanceTimeBy(2_000)
    waitForIdle()
    assertTrue(sheet.entries.isEmpty())
  }

  @Test
  fun closingSheetCannotRegainFocus() = runComposeUiTest {
    val sheet = Sheet()
    val returnedFocusRequester = FocusRequester()
    val refocusRequestVersion = mutableIntStateOf(0)

    setContent {
      SheetTestTheme {
        Box(Modifier.size(width = 400.dp, height = 800.dp)) {
          Box(
            Modifier.testTag(ReturnedFocusTag)
              .size(48.dp)
              .focusRequester(returnedFocusRequester)
              .focusable()
          )
          LaunchedEffect(sheet.acceptsInput) {
            if (!sheet.acceptsInput) {
              withFrameNanos {}
              returnedFocusRequester.requestFocus()
            }
          }
          LaunchedEffect(Unit) {
            sheet.present<Unit> {
              ClosingSheetContent(refocusRequestVersion = refocusRequestVersion.intValue)
            }
          }

          SheetOverlay(sheet)
        }
      }
    }

    waitUntil(timeoutMillis = 5_000) { sheet.entries.isNotEmpty() }
    waitForIdle()
    onNodeWithTag(SheetFocusTag).assertIsFocused()

    mainClock.autoAdvance = false
    onNodeWithTag(DismissTag).performTouchInput {
      down(center)
      up()
    }
    waitForIdle()
    repeat(2) { mainClock.advanceTimeByFrame() }
    waitForIdle()
    onNodeWithTag(ReturnedFocusTag).assertIsFocused()

    runOnIdle { refocusRequestVersion.intValue += 1 }
    waitForIdle()

    onNodeWithTag(SheetFocusTag).assertIsNotFocused()
    onNodeWithTag(ReturnedFocusTag).assertIsFocused()
  }

  @Test
  fun catchingDismissalKeepsReturnedFocusThroughNextDismissal() = runComposeUiTest {
    val sheet = Sheet()
    val returnedFocusRequester = FocusRequester()

    setContent {
      SheetTestTheme {
        Box(Modifier.size(width = 400.dp, height = 800.dp)) {
          Box(
            Modifier.testTag(ReturnedFocusTag)
              .size(48.dp)
              .focusRequester(returnedFocusRequester)
              .focusable()
          )
          LaunchedEffect(sheet.acceptsInput) {
            if (!sheet.acceptsInput) {
              withFrameNanos {}
              returnedFocusRequester.requestFocus()
            }
          }
          LaunchedEffect(Unit) { sheet.present<Unit> { ClosingSheetContent() } }

          SheetOverlay(sheet)
        }
      }
    }

    waitUntil(timeoutMillis = 5_000) { sheet.entries.isNotEmpty() }
    waitForIdle()
    onNodeWithTag(SheetFocusTag).assertIsFocused()

    mainClock.autoAdvance = false
    onNodeWithTag(DismissTag).performTouchInput {
      down(center)
      up()
    }
    waitForIdle()

    assertFalse(sheet.acceptsInput)
    repeat(2) { mainClock.advanceTimeByFrame() }
    waitForIdle()
    onNodeWithTag(ReturnedFocusTag).assertIsFocused()

    onNodeWithTag(SheetContentTag).performTouchInput {
      down(center)
      moveBy(Offset(0f, -80f), delayMillis = 500)
      up()
    }
    mainClock.autoAdvance = true
    waitForIdle()

    assertNotNull(sheet.entries.singleOrNull())
    assertTrue(sheet.acceptsInput)
    onNodeWithTag(ReturnedFocusTag).assertIsFocused()

    mainClock.autoAdvance = false
    onNodeWithTag(DismissTag).performTouchInput {
      down(center)
      up()
    }
    waitForIdle()

    assertFalse(sheet.acceptsInput)
    onNodeWithTag(ReturnedFocusTag).assertIsFocused()
  }

  @Composable
  context(scope: SheetScope<Unit>)
  private fun ClosingSheetContent(refocusRequestVersion: Int = 0) {
    val sheetFocusRequester = remember { FocusRequester() }

    Column(Modifier.testTag(SheetContentTag).size(width = 400.dp, height = 200.dp)) {
      Box(
        Modifier.testTag(SheetFocusTag).size(48.dp).focusRequester(sheetFocusRequester).focusable()
      )
      Box(Modifier.testTag(DismissTag).size(48.dp).clickable { scope.dismiss() })
    }
    LaunchedEffect(Unit) { sheetFocusRequester.requestFocus() }
    LaunchedEffect(refocusRequestVersion) {
      if (refocusRequestVersion > 0) {
        sheetFocusRequester.requestFocus()
      }
    }
  }

  @Test
  fun entryCompletingDuringEntranceAnimationIsResolved() = runComposeUiTest {
    val sheet = Sheet()
    var result: String? = null

    setContent {
      SheetTestTheme {
        Box(Modifier.size(width = 400.dp, height = 800.dp)) {
          LaunchedEffect(Unit) {
            result = sheet.present {
              LaunchedEffect(Unit) { complete("done") }
              Box(Modifier.size(200.dp))
            }
          }

          SheetOverlay(sheet)
        }
      }
    }

    waitUntil(timeoutMillis = 5_000) { result == "done" }

    assertEquals("done", result)
    assertEquals(0, sheet.entries.size)
  }

  @Composable
  private fun SheetTestTheme(content: @Composable () -> Unit) {
    CompositionLocalProvider(
      LocalAppColors provides LightColors,
      LocalAppShadows provides LightAppShadows,
      LocalThemeMode provides ResolvedThemeMode.Light,
      LocalHazeBlurStyle provides
        HazeBlurStyle(blurRadius = 20.dp, noiseFactor = 0f, colorEffects = listOf()),
      content = content,
    )
  }

  private companion object {
    const val SheetFocusTag = "sheet-focus"
    const val SheetContentTag = "sheet-content"
    const val DismissTag = "sheet-dismiss"
    const val ReturnedFocusTag = "returned-focus"
  }
}
