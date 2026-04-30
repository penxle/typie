package co.typie.ext

import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertIsFocused
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class ClickableFocusDesktopTest {
  @Test
  fun clickableDoesNotClearExistingFocus() = runComposeUiTest {
    var clicked = false

    setContent {
      val focusRequester = remember { FocusRequester() }

      Column {
        Box(Modifier.testTag(FocusTargetTag).size(48.dp).focusRequester(focusRequester).focusable())
        Box(Modifier.testTag(ClickTargetTag).size(48.dp).clickable { clicked = true })
      }

      LaunchedEffect(Unit) { focusRequester.requestFocus() }
    }

    waitForIdle()

    onNodeWithTag(ClickTargetTag).performClick()
    waitForIdle()

    assertTrue(clicked)
    onNodeWithTag(FocusTargetTag).assertIsFocused()
  }

  @Test
  fun combinedClickableDoesNotClearExistingFocus() = runComposeUiTest {
    var clicked = false

    setContent {
      val focusRequester = remember { FocusRequester() }

      Column {
        Box(Modifier.testTag(FocusTargetTag).size(48.dp).focusRequester(focusRequester).focusable())
        Box(
          Modifier.testTag(ClickTargetTag)
            .size(48.dp)
            .combinedClickable(onClick = { clicked = true }, onLongClick = {})
        )
      }

      LaunchedEffect(Unit) { focusRequester.requestFocus() }
    }

    waitForIdle()

    onNodeWithTag(ClickTargetTag).performClick()
    waitForIdle()

    assertTrue(clicked)
    onNodeWithTag(FocusTargetTag).assertIsFocused()
  }

  private companion object {
    const val FocusTargetTag = "focus-target"
    const val ClickTargetTag = "click-target"
  }
}
