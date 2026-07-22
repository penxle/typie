package co.typie.ui.component.dialog

import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
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
import kotlin.test.assertFalse
import kotlin.test.assertNotNull

@OptIn(ExperimentalTestApi::class)
class DialogOverlayDesktopTest {
  @Test
  fun dismissalDoesNotClearFocusOutsideDialog() = runComposeUiTest {
    val dialog = Dialog()
    val outsideFocusRequester = FocusRequester()
    var dismissRequest: (() -> Unit)? = null

    setContent {
      DialogTestTheme {
        Box(Modifier.size(400.dp)) {
          Box(
            Modifier.testTag(OutsideFocusTag)
              .size(48.dp)
              .focusRequester(outsideFocusRequester)
              .focusable()
          )
          LaunchedEffect(Unit) {
            outsideFocusRequester.requestFocus()
            dialog.present<Unit> {
              FocuslessDialogContent(onDismissReady = { dismissRequest = it })
            }
          }

          DialogOverlay(dialog)
        }
      }
    }

    waitUntil(timeoutMillis = 5_000) { dialog.current != null && dismissRequest != null }
    waitForIdle()
    onNodeWithTag(OutsideFocusTag).assertIsFocused()

    mainClock.autoAdvance = false
    runOnIdle { checkNotNull(dismissRequest).invoke() }
    waitForIdle()

    assertFalse(dialog.acceptsInput)
    onNodeWithTag(OutsideFocusTag).assertIsFocused()
  }

  @Test
  fun dismissalReleasesFocusBeforeRemoval() = runComposeUiTest {
    val dialog = Dialog()

    setContent {
      DialogTestTheme {
        Box(Modifier.size(400.dp)) {
          LaunchedEffect(Unit) { dialog.present<Unit> { ClosingDialogContent() } }

          DialogOverlay(dialog)
        }
      }
    }

    waitUntil(timeoutMillis = 5_000) { dialog.current != null }
    waitForIdle()
    onNodeWithTag(DialogFocusTag).assertIsFocused()

    mainClock.autoAdvance = false
    onNodeWithTag(DismissTag).performTouchInput {
      down(center)
      up()
    }
    waitForIdle()

    assertNotNull(dialog.current)
    assertFalse(dialog.acceptsInput)
    onNodeWithTag(DialogFocusTag).assertIsNotFocused()
  }

  @Composable
  context(scope: DialogScope<Unit>)
  private fun FocuslessDialogContent(onDismissReady: (() -> Unit) -> Unit) {
    SideEffect { onDismissReady(scope::dismiss) }
    Box(Modifier.size(200.dp))
  }

  @Composable
  context(scope: DialogScope<Unit>)
  private fun ClosingDialogContent() {
    val dialogFocusRequester = remember { FocusRequester() }

    Column(Modifier.size(200.dp)) {
      Box(
        Modifier.testTag(DialogFocusTag)
          .size(48.dp)
          .focusRequester(dialogFocusRequester)
          .focusable()
      )
      Box(Modifier.testTag(DismissTag).size(48.dp).clickable { scope.dismiss() })
    }
    LaunchedEffect(Unit) { dialogFocusRequester.requestFocus() }
  }

  @Composable
  private fun DialogTestTheme(content: @Composable () -> Unit) {
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
    const val OutsideFocusTag = "outside-focus"
    const val DialogFocusTag = "dialog-focus"
    const val DismissTag = "dialog-dismiss"
  }
}
