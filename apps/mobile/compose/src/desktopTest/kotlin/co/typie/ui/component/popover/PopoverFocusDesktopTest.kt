package co.typie.ui.component.popover

import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertIsFocused
import androidx.compose.ui.test.assertIsNotFocused
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.dp
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
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class PopoverFocusDesktopTest {
  @Test
  fun openingPopoverClearsExistingFocus() = runComposeUiTest {
    val overlayState = PopoverOverlayState()
    var focusWasAcquired = false
    var acceptsInputWhenFocusWasLost: Boolean? = null

    setContent {
      val focusRequester = remember { FocusRequester() }

      CompositionLocalProvider(LocalPopoverOverlayState provides overlayState) {
        Column {
          Box(
            Modifier.testTag(FocusTargetTag)
              .size(48.dp)
              .focusRequester(focusRequester)
              .onFocusChanged { state ->
                if (state.isFocused) {
                  focusWasAcquired = true
                } else if (focusWasAcquired) {
                  acceptsInputWhenFocusWasLost = overlayState.acceptsInput
                }
              }
              .focusable()
          )
          Popover(
            anchor = { Box(Modifier.testTag(PopoverAnchorTag).size(48.dp)) },
            pane = { Box(Modifier.size(48.dp)) },
          )
        }
      }

      LaunchedEffect(Unit) { focusRequester.requestFocus() }
    }

    waitForIdle()

    onNodeWithTag(PopoverAnchorTag).performTouchInput {
      down(center)
      up()
    }
    waitForIdle()

    onNodeWithTag(FocusTargetTag).assertIsNotFocused()
    assertEquals(true, acceptsInputWhenFocusWasLost)
  }

  @Test
  fun menuAdmitsSynchronousDestinationBeforeClosingPopover() = runComposeUiTest {
    val overlayState = PopoverOverlayState()
    val destinationAcceptsInput = mutableStateOf(false)
    val destinationFocusRequester = FocusRequester()
    var popoverAcceptedInputAtDestinationAdmission: Boolean? = null

    setContent {
      PopoverTestTheme {
        CompositionLocalProvider(LocalPopoverOverlayState provides overlayState) {
          Box(Modifier.size(400.dp)) {
            Box(
              Modifier.testTag(DestinationFocusTag)
                .size(48.dp)
                .focusRequester(destinationFocusRequester)
                .focusable()
            )
            PopoverMenu(anchor = { Box(Modifier.testTag(PopoverAnchorTag).size(48.dp)) }) {
              item(
                content = {
                  val modifier =
                    if (LocalPopoverPaneRenderPhase.current == PopoverPaneRenderPhase.Interactive) {
                      Modifier.testTag(MenuItemTag)
                    } else {
                      Modifier
                    }
                  Box(modifier.size(48.dp))
                }
              ) {
                popoverAcceptedInputAtDestinationAdmission = overlayState.acceptsInput
                destinationAcceptsInput.value = true
                destinationFocusRequester.requestFocus()
              }
            }
            PopoverOverlay(overlayState)
          }
        }
      }
    }

    onNodeWithTag(PopoverAnchorTag).performTouchInput {
      down(center)
      up()
    }
    waitForIdle()

    onNodeWithTag(MenuItemTag, useUnmergedTree = true).performTouchInput {
      down(center)
      up()
    }
    waitForIdle()

    assertEquals(true, popoverAcceptedInputAtDestinationAdmission)
    assertTrue(destinationAcceptsInput.value)
    onNodeWithTag(DestinationFocusTag).assertIsFocused()
  }

  @Composable
  private fun PopoverTestTheme(content: @Composable () -> Unit) {
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
    const val FocusTargetTag = "focus-target"
    const val PopoverAnchorTag = "popover-anchor"
    const val MenuItemTag = "menu-item"
    const val DestinationFocusTag = "destination-focus"
  }
}
