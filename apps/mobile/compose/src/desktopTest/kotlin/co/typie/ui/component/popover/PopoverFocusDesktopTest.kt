package co.typie.ui.component.popover

import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertIsNotFocused
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.dp
import kotlin.test.Test

@OptIn(ExperimentalTestApi::class)
class PopoverFocusDesktopTest {
  @Test
  fun openingPopoverClearsExistingFocus() = runComposeUiTest {
    setContent {
      val focusRequester = remember { FocusRequester() }
      val overlayState = remember { PopoverOverlayState() }

      CompositionLocalProvider(LocalPopoverOverlayState provides overlayState) {
        Column {
          Box(
            Modifier.testTag(FocusTargetTag).size(48.dp).focusRequester(focusRequester).focusable()
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
  }

  private companion object {
    const val FocusTargetTag = "focus-target"
    const val PopoverAnchorTag = "popover-anchor"
  }
}
