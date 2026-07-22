package co.typie.ui.component.dialog

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.hasScrollAction
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.swipeUp
import androidx.compose.ui.test.v2.runComposeUiTest
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
class DialogLayoutDesktopTest {
  @Test
  fun longMessageScrollsWhileHeaderAndActionsStayFixed() = runComposeUiTest {
    setContent {
      DialogLayoutTestTheme {
        Box(Modifier.testTag(RootTag).size(width = 280.dp, height = 240.dp)) {
          DialogLayout(
            title = Title,
            message = LongMessage,
            actions = { DialogActionButton(text = ConfirmText, onClick = {}) },
          )
        }
      }
    }

    val title = onNodeWithText(Title)
    val message = onNodeWithText(LongMessage)
    val action = onNodeWithText(ConfirmText)
    title.assertIsDisplayed()
    action.assertIsDisplayed()

    val scrollableBody = onNode(hasScrollAction())
    val titleBoundsBeforeScroll = title.fetchSemanticsNode().boundsInRoot
    val messageBoundsBeforeScroll = message.fetchSemanticsNode().boundsInRoot
    val actionBoundsBeforeScroll = action.fetchSemanticsNode().boundsInRoot

    scrollableBody.performTouchInput { swipeUp() }
    waitForIdle()

    val titleBoundsAfterScroll = title.fetchSemanticsNode().boundsInRoot
    val messageBoundsAfterScroll = message.fetchSemanticsNode().boundsInRoot
    val actionBoundsAfterScroll = action.fetchSemanticsNode().boundsInRoot
    assertEquals(titleBoundsBeforeScroll.top, titleBoundsAfterScroll.top, absoluteTolerance = 0.5f)
    assertTrue(messageBoundsAfterScroll.top < messageBoundsBeforeScroll.top)
    assertEquals(
      actionBoundsBeforeScroll.top,
      actionBoundsAfterScroll.top,
      absoluteTolerance = 0.5f,
    )
  }

  @Test
  fun shortMessageKeepsCompactHeight() = runComposeUiTest {
    setContent {
      DialogLayoutTestTheme {
        Box(Modifier.testTag(RootTag).size(width = 280.dp, height = 600.dp)) {
          DialogLayout(
            title = Title,
            message = "짧은 본문",
            actions = { DialogActionButton(text = ConfirmText, onClick = {}) },
          )
        }
      }
    }

    val rootBounds = onNodeWithTag(RootTag).fetchSemanticsNode().boundsInRoot
    val actionBounds = onNodeWithText(ConfirmText).fetchSemanticsNode().boundsInRoot
    assertTrue(actionBounds.bottom < rootBounds.center.y)
  }

  @Composable
  private fun DialogLayoutTestTheme(content: @Composable () -> Unit) {
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
    const val RootTag = "root"
    const val Title = "긴 내용"
    const val ConfirmText = "확인"
    val LongMessage = (1..30).joinToString("\n") { "본문 $it" }
  }
}
